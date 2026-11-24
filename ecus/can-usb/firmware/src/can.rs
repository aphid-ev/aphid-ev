use crate::board::Transceiver;
use crate::{EventSender, FrameReceiver, leds};
use can_usb_protocol::{BusError, BusEvent, Forwarded, Frame, OverrunCounter};
use core::future::poll_fn;
use defmt::{debug, error, trace, unwrap, warn};
use embassy_executor::Spawner;
use embassy_futures::select::{Either, select};
use embassy_stm32::can::config::{GlobalFilter, NonMatchingFilter, TxBufferMode};
use embassy_stm32::can::{CanConfigurator, CanRx, CanTx, OperatingMode, filter};
use embassy_time::Instant;
use embedded_can::Frame as _;

pub struct CanBus {
    can_config: CanConfigurator<'static>,
    transceiver: Transceiver,
    event_sender: EventSender,
    frame_receiver: FrameReceiver,
}

impl CanBus {
    pub fn new(
        can: CanConfigurator<'static>,
        transceiver: Transceiver,
        event_sender: EventSender,
        frame_receiver: FrameReceiver,
    ) -> Self {
        Self {
            can_config: can,
            transceiver,
            event_sender,
            frame_receiver,
        }
    }

    /// Takes the transceiver out of standby, starts FDCAN1 and spawns the
    /// receive and transmit tasks. The caller holds the returned
    /// [`Transceiver`] for as long as the bus runs: dropping it puts the
    /// transceiver back in standby.
    #[must_use = "dropping the transceiver puts it in standby"]
    pub fn start(mut self, spawner: Spawner) -> Transceiver {
        // Every standard and every extended identifier goes to RX FIFO 0.
        // Remote frames are rejected by the controller: the protocol's
        // Frame cannot say a frame is remote, so passing one on would hand
        // the host a data frame that was never sent.
        let properties = self.can_config.properties();
        properties.set_standard_filter(
            filter::StandardFilterSlot::_0,
            filter::StandardFilter::accept_all_into_fifo0(),
        );
        properties.set_extended_filter(
            filter::ExtendedFilterSlot::_0,
            filter::ExtendedFilter::accept_all_into_fifo0(),
        );
        let global_filter = GlobalFilter::default()
            .set_handle_standard_frames(NonMatchingFilter::IntoRxFifo0)
            .set_handle_extended_frames(NonMatchingFilter::IntoRxFifo0)
            .set_reject_remote_standard_frames(true)
            .set_reject_remote_extended_frames(true);
        // The TX buffers send in the order they were written. In this mode
        // the driver's write() waits for a free buffer instead of evicting
        // a queued frame of lower priority.
        let config = self
            .can_config
            .config()
            .set_global_filter(global_filter)
            .set_tx_buffer_mode(TxBufferMode::Fifo);
        self.can_config.set_config(config);
        self.can_config.set_bitrate(500_000);
        // The FDCAN pins are configured, so the transceiver may leave
        // standby before FDCAN1 leaves init.
        self.transceiver.wake();
        let can = self.can_config.start(OperatingMode::NormalOperationMode);
        let (can_tx, can_rx, _) = can.split();

        // Spawn failure means the single pool slot is already taken, i.e.
        // start() was called twice. That's a programming error and the
        // device is useless without these tasks, so panic via defmt.
        spawner.spawn(unwrap!(rx_task(can_rx, self.event_sender)));
        spawner.spawn(unwrap!(tx_task(can_tx, self.frame_receiver)));

        self.transceiver
    }
}

/// Reads the RX FIFO and queues what it reads for the host.
///
/// It never waits for room in the event queue. FDCAN1's RX FIFO holds three
/// frames, and while this task waited it would fill and drop frames without
/// a trace. Events that do not fit are counted by the [`OverrunCounter`]
/// and reported as one `Overrun` as soon as there is room: ahead of the next
/// event, or by itself once the USB side takes an event out of the queue.
#[embassy_executor::task]
async fn rx_task(mut can_rx: CanRx<'static>, event_sender: EventSender) {
    let mut overrun = OverrunCounter::new();

    loop {
        let read = if overrun.is_pending() {
            let room = poll_fn(|cx| event_sender.poll_ready_to_send(cx));
            // read() is cancel-safe: it takes a frame out of the FIFO only
            // in the poll that returns it.
            match select(can_rx.read(), room).await {
                Either::First(read) => read,
                Either::Second(()) => {
                    if overrun.flush(|event| event_sender.try_send(event)) {
                        debug!("Overrun reported");
                    }
                    continue;
                }
            }
        } else {
            can_rx.read().await
        };

        let event = match read {
            Ok(envelope) => {
                // The filters reject remote frames, so this only guards
                // against a change to them.
                if envelope.frame.is_remote_frame() {
                    trace!("Ignored a remote frame");
                    continue;
                }
                leds::RX.trigger();
                let frame = Frame::from_can(&envelope.frame);
                trace!("Received frame: {}", frame);
                BusEvent::Frame {
                    timestamp_us: timestamp_us(envelope.ts),
                    frame,
                }
            }
            Err(error) => {
                let timestamp_us = timestamp_us(Instant::now());
                leds::ERR.trigger();
                let error = bus_error(error);
                error!("CAN read error: {}", error);
                BusEvent::BusError {
                    timestamp_us,
                    error,
                }
            }
        };

        let was_pending = overrun.is_pending();
        if overrun.forward(event, |event| event_sender.try_send(event)) == Forwarded::Lost {
            match event {
                BusEvent::Frame { .. } if !was_pending => {
                    warn!("Event queue full, counting lost frames")
                }
                BusEvent::Frame { .. } => trace!("Event queue full, lost {}", event),
                _ => warn!("Event queue full, lost {}", event),
            }
        }
    }
}

/// Sends the host's frames in the order they arrive. write() waits while
/// all three TX buffers are taken, which in turn fills the frame queue and
/// holds back the USB side.
#[embassy_executor::task]
async fn tx_task(mut can_tx: CanTx<'static>, frame_receiver: FrameReceiver) {
    loop {
        let frame = frame_receiver.receive().await;

        trace!("Sending frame: {}", frame);

        match frame.to_can::<embassy_stm32::can::Frame>() {
            Some(can_frame) => {
                // In FIFO mode the driver never evicts a queued frame, so
                // this is always None. Say so loudly if that ever changes.
                if let Some(evicted) = can_tx.write(&can_frame).await {
                    error!("TX buffer evicted {}", Frame::from_can(&evicted));
                }
                leds::TX.trigger();
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
