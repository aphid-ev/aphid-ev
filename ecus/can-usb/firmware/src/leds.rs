//! The status LEDs, as retriggerable pulses.
//!
//! A task that sees a frame or an error calls [`Pulse::trigger`], which only
//! signals the LED's own task and returns. That task lights the LED and
//! keeps it lit until no trigger has come for the pulse length, so at full
//! bus load the LED stays on and the CAN tasks never write a GPIO.

use defmt::unwrap;
use embassy_executor::Spawner;
use embassy_futures::select::{Either, select};
use embassy_stm32::gpio::Output;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Timer};

/// Lit for this long after each received frame.
const RX_PULSE: Duration = Duration::from_millis(20);
/// Lit for this long after each transmitted frame.
const TX_PULSE: Duration = Duration::from_millis(20);
/// Lit for this long after each bus error.
const ERR_PULSE: Duration = Duration::from_millis(500);

/// Received a frame.
pub static RX: Pulse = Pulse::new();
/// Handed a frame to the transmit mailbox.
pub static TX: Pulse = Pulse::new();
/// Saw a bus error.
pub static ERR: Pulse = Pulse::new();

/// The trigger of one LED.
pub struct Pulse {
    signal: Signal<CriticalSectionRawMutex, ()>,
}

impl Pulse {
    const fn new() -> Self {
        Self {
            signal: Signal::new(),
        }
    }

    /// Lights the LED, or keeps it lit, for one more pulse length.
    pub fn trigger(&self) {
        self.signal.signal(());
    }
}

/// Spawns one task per LED.
pub fn start(spawner: Spawner, rx: Output<'static>, tx: Output<'static>, err: Output<'static>) {
    // As in CanBus::start, spawn failure means start() ran twice.
    spawner.spawn(unwrap!(pulse_task(rx, &RX, RX_PULSE)));
    spawner.spawn(unwrap!(pulse_task(tx, &TX, TX_PULSE)));
    spawner.spawn(unwrap!(pulse_task(err, &ERR, ERR_PULSE)));
}

#[embassy_executor::task(pool_size = 3)]
async fn pulse_task(mut led: Output<'static>, pulse: &'static Pulse, length: Duration) {
    loop {
        pulse.signal.wait().await;
        led.set_high();
        // Every trigger during the pulse restarts it.
        while let Either::Second(()) = select(Timer::after(length), pulse.signal.wait()).await {}
        led.set_low();
    }
}
