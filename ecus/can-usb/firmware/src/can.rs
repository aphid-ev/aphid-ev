use crate::{RxSender, TxReceiver};
use can_usb_protocol::{BusError, BusEvent, Frame};
use defmt::{debug, error, unwrap, warn};
use embassy_executor::Spawner;
use embassy_stm32::can::{CanConfigurator, CanRx, CanTx, OperatingMode, filter};
use embassy_time::Instant;

pub struct CanBus {
    can_config: CanConfigurator<'static>,
    rx_sender: RxSender,
    tx_receiver: TxReceiver,
}

impl CanBus {
    pub fn new(
        can: CanConfigurator<'static>,
        rx_sender: RxSender,
        tx_receiver: TxReceiver,
    ) -> Self {
        Self {
            can_config: can,
            rx_sender,
            tx_receiver,
        }
    }

    pub fn start(mut self, spawner: Spawner) {
        self.can_config.properties().set_standard_filter(
            filter::StandardFilterSlot::_0,
            filter::StandardFilter::accept_all_into_fifo0(),
        );
        self.can_config.set_bitrate(500_000);
        let can = self.can_config.start(OperatingMode::NormalOperationMode);
        let (can_tx, can_rx, _) = can.split();

        // Spawn failure means the single pool slot is already taken, i.e.
        // start() was called twice. That's a programming error and the
        // device is useless without these tasks, so panic via defmt.
        spawner.spawn(unwrap!(rx_task(can_rx, self.rx_sender)));
        spawner.spawn(unwrap!(tx_task(can_tx, self.tx_receiver)));
    }
}

#[embassy_executor::task]
async fn rx_task(mut can_rx: CanRx<'static>, rx_sender: RxSender) {
    loop {
        match can_rx.read().await {
            Ok(envelope) => {
                let frame = Frame::from_can(&envelope.frame);
                debug!("Received frame: {}", frame);
                rx_sender
                    .send(BusEvent::Frame {
                        timestamp_us: timestamp_us(envelope.ts),
                        frame,
                    })
                    .await;
            }
            Err(error) => {
                let timestamp_us = timestamp_us(Instant::now());
                let error = bus_error(error);
                error!("CAN read error: {}", error);
                rx_sender
                    .send(BusEvent::BusError {
                        timestamp_us,
                        error,
                    })
                    .await;
            }
        }
    }
}

#[embassy_executor::task]
async fn tx_task(mut can_tx: CanTx<'static>, tx_receiver: TxReceiver) {
    loop {
        let frame = tx_receiver.receive().await;

        debug!("Sending frame: {}", frame);

        match frame.to_can::<embassy_stm32::can::Frame>() {
            // write() hands back any lower-priority frame it evicted from the
            // mailbox to make room. That frame never reaches the bus, so say
            // so rather than discarding it silently.
            Some(can_frame) => {
                if let Some(dropped) = can_tx.write(&can_frame).await {
                    warn!("TX mailbox full, dropped {}", Frame::from_can(&dropped));
                }
            }
            // Unreachable while Frame validates its identifier and payload on
            // construction, which is exactly why it should be loud if it ever
            // stops being true.
            None => error!("Refusing to send unrepresentable frame: {}", frame),
        }
    }
}

/// The protocol's timestamp: the embassy clock in microseconds, wrapping at
/// 2^32, with 1 us resolution from the 1 MHz tick.
///
/// A received frame's `envelope.ts` is in effect the moment this firmware
/// read it out of the FDCAN RX FIFO, so it lags the frame's arrival by the
/// time the frame waited there. embassy-stm32 0.6 tries to correct it with
/// the FDCAN timestamp counter, but that counter counts bit times while the
/// driver converts it as time quanta, so the correction is about a thousand
/// times too small to matter.
fn timestamp_us(instant: Instant) -> u32 {
    instant.as_micros() as u32
}

/// The orphan rule keeps this mapping out of `can-usb-protocol`, which
/// cannot depend on embassy-stm32.
fn bus_error(error: embassy_stm32::can::enums::BusError) -> BusError {
    use embassy_stm32::can::enums::BusError as Hal;

    match error {
        Hal::Stuff => BusError::Stuff,
        Hal::Form => BusError::Form,
        Hal::Acknowledge => BusError::Acknowledge,
        Hal::BitRecessive => BusError::BitRecessive,
        Hal::BitDominant => BusError::BitDominant,
        Hal::Crc => BusError::Crc,
        Hal::Software => BusError::Software,
    }
}
