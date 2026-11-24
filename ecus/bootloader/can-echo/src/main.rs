//! Minimal application used to test flashing a device through the CAN
//! bootloader: small enough to transfer quickly, and obviously alive once
//! running, since it echoes back every frame it receives.
//!
//! A frame on `boot_protocol::BUS_TEST_ID` is answered on
//! `boot_protocol::BUS_TEST_ECHO_ID` with the same data, so a node streaming
//! test frames never ties with the echo in arbitration. Every other frame
//! goes back unchanged.
//!
//! It links at the application offset rather than the start of flash — see
//! memory.x — so it only runs when started by the bootloader, and only once
//! the image tool has sealed its header.

#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_stm32::can::{
    CanConfigurator, Frame, IT0InterruptHandler, IT1InterruptHandler, OperatingMode, filter,
};
use embassy_stm32::time::Hertz;
use embassy_stm32::{Config, bind_interrupts, peripherals::*};
use embedded_can::{Id, StandardId};
use panic_halt as _;

// The header the bootloader checks before it starts this image, at the
// offset memory.x reserves for it.
boot_protocol::image_header!();

bind_interrupts!(struct CanIrqs {
    FDCAN1_IT0 => IT0InterruptHandler<FDCAN1>;
    FDCAN1_IT1 => IT1InterruptHandler<FDCAN1>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let mut config = Config::default();
    {
        use embassy_stm32::rcc::*;
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
        // Range 1 boost mode. Not needed at 144 MHz, which is within Range 1
        // normal mode; kept to match the CAN/USB firmware.
        config.rcc.boost = true;
        config.rcc.mux.fdcansel = mux::Fdcansel::PLL1_Q;
    }
    let p = embassy_stm32::init(config);

    let mut can_config = CanConfigurator::new(p.FDCAN1, p.PB8, p.PB9, CanIrqs);

    can_config.properties().set_standard_filter(
        filter::StandardFilterSlot::_0,
        filter::StandardFilter::accept_all_into_fifo0(),
    );
    can_config.set_bitrate(500_000);
    let mut can = can_config.start(OperatingMode::NormalOperationMode);

    loop {
        if let Ok(envelope) = can.read().await {
            // write returns any frame it evicted from the mailbox to make
            // room. Dropped on purpose here: this image is deliberately
            // minimal and has no logging, and an echo lost under load says
            // nothing about whether the flashing worked.
            let _dropped = can.write(&echo(envelope.frame)).await;
        }
    }
}

/// The answer to `frame`: the same data, on the identifier
/// `boot_protocol::echo_id` gives for a standard one.
fn echo(mut frame: Frame) -> Frame {
    if let Id::Standard(id) = *frame.id()
        && let Some(echo) = StandardId::new(boot_protocol::echo_id(id.as_raw()))
    {
        *frame.id_mut() = Id::Standard(echo);
    }

    frame
}
