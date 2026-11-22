//! Minimal application used to test flashing a device through the CAN
//! bootloader: small enough to transfer quickly, and obviously alive once
//! running, since it echoes back every frame it receives.
//!
//! It links at the application offset rather than the start of flash — see
//! memory.x — so it only runs when started by the bootloader.

#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_stm32::can::{
    filter, CanConfigurator, IT0InterruptHandler, IT1InterruptHandler, OperatingMode,
};
use embassy_stm32::time::Hertz;
use embassy_stm32::{bind_interrupts, peripherals::*, Config};
use panic_halt as _;

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
            let _dropped = can.write(&envelope.frame).await;
        }
    }
}
