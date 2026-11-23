#![no_std]
#![no_main]

pub mod app;
mod can;
mod crc;
mod flash;

use app::App;
use boot_protocol::{
    Block, Command, Error, Frame, MAX_BLOCK_LEN, NodeId, WAKEUP_WINDOW_MS, check_program,
};
use can::Can;
use cortex_m::peripheral::syst::SystClkSource;
use cortex_m_rt::entry;
use crc::Crc;
use flash::{Flash, FlashLayout};
use pac::{GPIOB, PWR, RCC, SCB, SYST};
use panic_halt as _;

#[cfg(all(feature = "stm32g431cx", feature = "stm32g491rx"))]
compile_error!("select exactly one chip feature: stm32g431cx or stm32g491rx");

#[cfg(not(any(feature = "stm32g431cx", feature = "stm32g491rx")))]
compile_error!("select a chip feature: stm32g431cx or stm32g491rx");

/// Peripherals for the chip being built for.
///
/// Every module reaches the hardware through this alias, so each build is
/// monomorphic over one chip and nothing has to be generic across them.
/// Supporting another chip is a feature plus a flash layout, not a change to
/// any code that touches peripherals.
#[cfg(feature = "stm32g431cx")]
pub use stm32g4::stm32g431 as pac;
#[cfg(feature = "stm32g491rx")]
pub use stm32g4::stm32g491 as pac;

const NODE_ID: NodeId = NodeId::MASTER;

/// 128K in 2K pages, single bank.
#[cfg(feature = "stm32g431cx")]
const FLASH_LAYOUT: FlashLayout = FlashLayout::new(0x08000000, 0x20000, 2048, BOOTLOADER_SIZE);

/// 512K in 4K pages, single bank. The larger page is not a typo: this part
/// has no bank select bit and PNB is seven bits wide, so 512K has to be 128
/// pages of 4K rather than 256 of 2K.
#[cfg(feature = "stm32g491rx")]
const FLASH_LAYOUT: FlashLayout = FlashLayout::new(0x08000000, 0x80000, 4096, BOOTLOADER_SIZE);

/// Must match the FLASH region in memory.x, and be a whole number of pages
/// on every supported chip.
const BOOTLOADER_SIZE: u32 = 8 * 1024;

enum State {
    Init,
    /// Waiting for **Catch bootloader**. `valid` is whether the image passed
    /// validation in `Init`; nothing writes flash before this state ends.
    Wait {
        valid: bool,
    },
    Bootloader,
    /// **Start application** received: validate the image again, since the
    /// programmer has usually just rewritten it.
    Start,
    /// Only entered with an image that has just passed validation.
    Application,
    Erase,
    Write {
        offset: u32,
        len: u16,
        crc: u16,
    },
}

#[entry]
fn main() -> ! {
    let p = pac::Peripherals::take().unwrap();
    let cp = pac::CorePeripherals::take().unwrap();

    let mut rcc = p.RCC;
    let mut pwr = p.PWR;
    let mut gpiob = p.GPIOB;
    let mut systick = cp.SYST;
    let mut scb = cp.SCB;

    let app = App::new(&FLASH_LAYOUT);
    let mut flash = Flash::new(p.FLASH);

    init_clock(&mut rcc, &mut pwr, &mut flash);

    let mut crc = Crc::new(p.CRC, &mut rcc);
    let mut can = Can::new(p.FDCAN, &mut rcc, &mut gpiob);

    let mut state = State::Init;
    loop {
        state = match state {
            State::Init => state_init(&mut can, &app, &mut crc),
            State::Wait { valid } => state_wait(&mut can, &mut systick, valid),
            State::Bootloader => state_bootloader(&mut can, &app),
            State::Start => state_start(&mut can, &app, &mut crc),
            State::Application => state_application(&mut can, &mut rcc, &mut gpiob, &mut scb, app),
            // Both of these answer the programmer, whether they worked or
            // not, so it is never left waiting on a command that finished.
            State::Erase => {
                let result = app.erase(&mut flash).map_err(Error::from);
                can.send(&Frame::response(result)).ok();
                State::Bootloader
            }
            State::Write { offset, len, crc } => {
                let result = state_write(&mut can, &mut flash, &app, offset, len, crc);
                can.send(&Frame::response(result)).ok();
                State::Bootloader
            }
        };
    }
}

fn init_clock(rcc: &mut RCC, pwr: &mut PWR, flash: &mut Flash) {
    // Enable clock for sysconfig and power peripherals
    rcc.apb2enr().modify(|_, w| w.syscfgen().set_bit());
    rcc.apb1enr1().modify(|_, w| w.pwren().set_bit());

    // Enable power boost mode (recomended above 150 MHz)
    pwr.cr5().modify(|_, w| w.r1mode().clear_bit());

    // Set FLASH latency before increasing sysclk
    flash.set_latency(5);

    // Enable the crystal oscillator and wait for it to be stable. The PLL
    // must not start from it before HSERDY is set.
    rcc.cr().modify(|_, w| w.hseon().set_bit());
    while rcc.cr().read().hserdy().is_not_ready() {}

    // Configure PLL with HSE input
    //   PLL Source = HSE (16 MHz)
    //   PLLM = /4
    //   PLLN = *72
    //   PLLR = /2  -> 144 MHz
    //   PLLQ = /6  ->  48 MHz
    rcc.pllcfgr().modify(|_, w| {
        w.pllsrc()
            .hse()
            .pllm()
            .div4()
            .plln()
            .div72()
            .pllr()
            .div2()
            .pllq()
            .div6()
    });

    // Enable PLL and wait for it to stabilize
    rcc.cr().modify(|_, w| w.pllon().set_bit());
    while rcc.cr().read().pllrdy().is_not_ready() {}

    // Set all bus prescalers to /1
    rcc.cfgr()
        .modify(|_, w| w.hpre().div1().ppre1().div1().ppre2().div1());

    // Switch sysclock to PLL and wait for it to be switched
    rcc.cfgr().modify(|_, w| w.sw().pll());
    while !rcc.cfgr().read().sws().is_pll() {}
}

fn state_init(can: &mut Can, app: &App, crc: &mut Crc) -> State {
    let image = app.validate(crc);

    let wakeup = Frame::wakeup(NODE_ID, &image);
    can.send(&wakeup).ok();

    State::Wait {
        valid: image.is_ok(),
    }
}

/// Without a valid image the node stays in the bootloader when the window
/// closes, so it can still be flashed.
fn state_wait(can: &mut Can, systick: &mut SYST, valid: bool) -> State {
    let mut milliseconds = 0u32;

    systick.set_clock_source(SystClkSource::Core);
    systick.set_reload(144_000); // Will get 1ms tick @ 144 MHz SYSCLK
    systick.clear_current();
    systick.enable_counter();

    loop {
        if let Some(frame) = can.receive()
            && frame.is_catch(NODE_ID)
        {
            systick.disable_counter();
            return State::Bootloader;
        };

        if systick.has_wrapped() {
            milliseconds += 1;

            if milliseconds > WAKEUP_WINDOW_MS {
                systick.disable_counter();
                return if valid {
                    State::Application
                } else {
                    State::Bootloader
                };
            }
        }
    }
}

fn state_bootloader(can: &mut Can, app: &App) -> State {
    loop {
        if let Some(frame) = can.receive() {
            match frame.as_command(NODE_ID) {
                Some(Ok(Command::Start)) => return State::Start,
                Some(Ok(Command::Erase)) => return State::Erase,
                Some(Ok(Command::Program { offset, len, crc })) => {
                    match check_program(offset, len, app.flash.application.len()) {
                        Ok(()) => return State::Write { offset, len, crc },
                        Err(error) => {
                            can.send(&Frame::response(Err(error))).ok();
                        }
                    }
                }
                // A command meant for us that we refuse, such as an erase
                // carrying the wrong key.
                Some(Err(error)) => {
                    can.send(&Frame::response(Err(error))).ok();
                }
                None => {}
            }
        }
    }
}

/// A valid image is started without a response, as before. An invalid one
/// is refused, and the node stays in the bootloader.
fn state_start(can: &mut Can, app: &App, crc: &mut Crc) -> State {
    match app.validate(crc) {
        Ok(_) => State::Application,
        Err(_) => {
            can.send(&Frame::response(Err(Error::InvalidImage))).ok();
            State::Bootloader
        }
    }
}

fn state_write(
    can: &mut Can,
    flash: &mut Flash,
    app: &App,
    offset: u32,
    len: u16,
    crc: u16,
) -> boot_protocol::Result<()> {
    let mut block: Block<MAX_BLOCK_LEN> = Block::new(offset, len as usize)?;

    loop {
        if let Some(frame) = can.receive()
            && let Some(sub_block) = frame.as_block_data(NODE_ID)
            && block.insert(&sub_block)?
        {
            block.validate(crc)?;
            break;
        }
    }

    app.write(flash, &block)?;

    Ok(())
}

fn state_application(
    can: &mut Can,
    rcc: &mut RCC,
    gpiob: &mut GPIOB,
    scb: &mut SCB,
    app: App,
) -> ! {
    can.reset(rcc, gpiob);

    // TODO: De-init other peripherals

    app.bootload(scb)
}
