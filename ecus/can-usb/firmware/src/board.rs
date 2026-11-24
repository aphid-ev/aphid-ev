//! The board this firmware runs on: `can-usb-hw-revA`, the KiCad project in
//! `ecus/can-usb/hardware`. No other revision is supported.
//!
//! This is the only module that names pins. Everything else gets its
//! peripherals from [`Board`].
//!
//! | Signal | Pin | Notes |
//! | --- | --- | --- |
//! | USB D- / D+ | PA11 / PA12 | |
//! | FDCAN1 RX / TX | PB8 / PB9 | PB8 is also BOOT0, see the option bytes on the wiki |
//! | `CAN_STB` | PC13 | TJA1042 standby, pulled up on the board, high = standby |
//! | `LED_RX` | PA7 | green, high = on |
//! | `LED_TX` | PA6 | green, high = on |
//! | `LED_ERR` | PA5 | red, high = on |
//! | HSE | PF0 / PF1 | 16 MHz crystal |

use embassy_stm32::Peri;
use embassy_stm32::can::{CanConfigurator, IT0InterruptHandler, IT1InterruptHandler};
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_stm32::peripherals::{FDCAN1, PB8, PB9, USB};
use embassy_stm32::time::Hertz;
use embassy_stm32::usb::{self, Driver};
use embassy_stm32::{Config, Peripherals, bind_interrupts};

bind_interrupts!(struct UsbIrqs {
    USB_LP => usb::InterruptHandler<USB>;
});

bind_interrupts!(struct CanIrqs {
    FDCAN1_IT0 => IT0InterruptHandler<FDCAN1>;
    FDCAN1_IT1 => IT1InterruptHandler<FDCAN1>;
});

/// Clocks for the 16 MHz crystal: 144 MHz system clock, 48 MHz for USB and
/// FDCAN.
pub fn config() -> Config {
    use embassy_stm32::rcc::*;

    let mut config = embassy_stm32::Config::default();
    config.rcc.hse = Some(Hse {
        freq: Hertz(16_000_000),
        mode: HseMode::Oscillator,
    });
    config.rcc.pll = Some(Pll {
        source: PllSource::HSE,
        prediv: PllPreDiv::DIV4,
        mul: PllMul::MUL72,
        divp: None,
        divr: Some(PllRDiv::DIV2), // PLLR -> 144 MHz
        divq: Some(PllQDiv::DIV6), // PLLQ -> 48 MHz
    });
    config.rcc.sys = Sysclk::PLL1_R;
    // Range 1 boost mode. Not strictly needed at 144 MHz — that is within
    // Range 1 normal mode, and embassy picks the same four flash wait
    // states either way — but it leaves headroom towards the 170 MHz
    // ceiling. Keep in sync with can_bootloader, which sets PWR_CR5
    // by hand.
    config.rcc.boost = true;
    config.rcc.mux.clk48sel = mux::Clk48sel::PLL1_Q;
    config.rcc.mux.fdcansel = mux::Fdcansel::PLL1_Q;
    config
}

/// The peripherals of the board, set up and named by what they do.
pub struct Board {
    pub usb: Driver<'static, USB>,
    pub can: CanPeripherals,
    pub transceiver: Transceiver,
    pub led_rx: Output<'static>,
    pub led_tx: Output<'static>,
    pub led_err: Output<'static>,
}

impl Board {
    /// Takes the peripherals from `embassy_stm32::init(board::config())`.
    ///
    /// The transceiver stays in standby: STB is driven high, as the pull-up
    /// already holds it, before the FDCAN pins are configured. Only
    /// [`Transceiver::wake`] releases it.
    pub fn new(p: Peripherals) -> Self {
        // PC13 is a backup-domain pin that can only source a few mA. It
        // drives nothing but the TJA1042's STB input, so low speed is enough.
        let transceiver = Transceiver {
            stb: Output::new(p.PC13, Level::High, Speed::Low),
        };

        Self {
            usb: Driver::new(p.USB, UsbIrqs, p.PA12, p.PA11),
            can: CanPeripherals {
                fdcan: p.FDCAN1,
                rx: p.PB8,
                tx: p.PB9,
            },
            transceiver,
            led_rx: Output::new(p.PA7, Level::Low, Speed::Low),
            led_tx: Output::new(p.PA6, Level::Low, Speed::Low),
            led_err: Output::new(p.PA5, Level::Low, Speed::Low),
        }
    }
}

/// FDCAN1 and its pins, from which the CAN task builds a fresh controller
/// each time it starts the bus.
pub struct CanPeripherals {
    fdcan: Peri<'static, FDCAN1>,
    rx: Peri<'static, PB8>,
    tx: Peri<'static, PB9>,
}

impl CanPeripherals {
    /// Resets FDCAN1, connects its pins and returns it in configuration
    /// mode. embassy-stm32 0.6 cannot take a started controller back to
    /// configuration mode, so the bus is stopped by dropping the controller
    /// and every part split from it, which turns FDCAN1's clock off and
    /// disconnects its pins, and started again from here.
    pub fn configurator(&mut self) -> CanConfigurator<'_> {
        CanConfigurator::new(
            self.fdcan.reborrow(),
            self.rx.reborrow(),
            self.tx.reborrow(),
            CanIrqs,
        )
    }
}

/// The TJA1042's standby input.
///
/// In standby the transceiver neither drives the bus nor passes received
/// bits to FDCAN1. Dropping this releases PC13, and the board's pull-up puts
/// the transceiver back in standby.
pub struct Transceiver {
    stb: Output<'static>,
}

impl Transceiver {
    /// Leaves standby: the transceiver sends and receives. Call it after
    /// FDCAN1 is configured and before it leaves init.
    pub fn wake(&mut self) {
        self.stb.set_low();
    }

    /// Enters standby. Call it whenever the bus is stopped.
    pub fn standby(&mut self) {
        self.stb.set_high();
    }
}
