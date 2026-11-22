//! VCM firmware for the monitor MCU, STM32F091CC.
//!
//! A starting point only: it brings up embassy on the default clocks and
//! logs a heartbeat. It drives no pins, so it runs on any board revision.
//! Behaviour belongs in `vcm-core` and `ev-can`; this binary will wire
//! their logic to the peripherals.

#![no_std]
#![no_main]

use defmt::info;
use embassy_executor::Spawner;
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};

// Linked so every firmware build checks that the shared crates compile for
// this target. Drop these once the logic is wired in.
use {ev_can as _, vcm_core as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let _peripherals = embassy_stm32::init(Default::default());

    info!("VCM monitor MCU started");

    loop {
        Timer::after_secs(1).await;
        info!("alive");
    }
}
