//! The CAN task: the only owner of FDCAN1 and the transceiver.
//!
//! It runs the bus as the latest [`BusCommand`] from the USB control
//! handler says. Each command stops the bus, clears the queues as the
//! command says, and starts the bus again with the command's config if it
//! is to run. While the bus runs, receiving and transmitting are two loops
//! in this task, and a new command ends both.

use crate::board::{CanPeripherals, Transceiver};
use crate::{ClearSignal, CommandSignal, EventSender, FrameReceiver, leds};
use can_usb_protocol::{
    Bitrate, BusCommand, BusError, BusEvent, Config, Forwarded, Frame, Mode, OverrunCounter,
};
use core::convert::Infallible;
use core::future::{pending, poll_fn};
use defmt::{debug, error, info, trace, unwrap, warn};
use embassy_executor::Spawner;
use embassy_futures::select::{Either, Either3, select, select3};
use embassy_stm32::can::config::{GlobalFilter, NonMatchingFilter, TxBufferMode};
use embassy_stm32::can::{CanConfigurator, CanRx, CanTx, OperatingMode, filter};
use embassy_time::Instant;
use embedded_can::Frame as _;

pub struct CanBus {
    peripherals: CanPeripherals,
    transceiver: Transceiver,
    event_sender: EventSender,
    frame_receiver: FrameReceiver,
    commands: &'static CommandSignal,
    events_cleared: &'static ClearSignal,
    frames_cleared: &'static ClearSignal,
}

impl CanBus {
    /// `events_cleared` and `frames_cleared` are raised after the task
    /// clears the event and the frame queue, so that the USB tasks drop what
    /// they had already taken out of them.
    pub fn new(
        peripherals: CanPeripherals,
        transceiver: Transceiver,
        event_sender: EventSender,
        frame_receiver: FrameReceiver,
        commands: &'static CommandSignal,
        events_cleared: &'static ClearSignal,
        frames_cleared: &'static ClearSignal,
    ) -> Self {
        Self {
            peripherals,
            transceiver,
            event_sender,
            frame_receiver,
            commands,
            events_cleared,
            frames_cleared,
        }
    }

    /// Spawns the CAN task, which starts the bus with
    /// [`BusCommand::POWER_ON`].
    pub fn start(self, spawner: Spawner) {
        // Spawn failure means the single pool slot is already taken, i.e.
        // start() was called twice. That's a programming error and the
        // device is useless without this task, so panic via defmt.
        spawner.spawn(unwrap!(can_task(self)));
    }

    /// Clears what `command` says to clear. The bus is stopped.
    fn clear(&mut self, command: BusCommand, overrun: &mut OverrunCounter) {
        if command.clear_events {
            self.event_sender.clear();
            *overrun = OverrunCounter::new();
            self.events_cleared.signal(());
        }

        // Frames queued for one bit rate or mode must not go out in the
        // next. Up to three more were in FDCAN1's transmit buffers; the
        // reset when the bus starts again clears those without a count.
        let mut discarded = 0u32;
        while self.frame_receiver.try_receive().is_ok() {
            discarded += 1;
        }
        self.frames_cleared.signal(());
        if discarded > 0 {
            info!("Discarded {} queued frames", discarded);
        }
    }

    /// Runs the bus with `config` until the next command, and returns it.
    async fn run(&mut self, config: Config, overrun: &mut OverrunCounter) -> BusCommand {
        let (commands, event_sender, frame_receiver) =
            (self.commands, self.event_sender, self.frame_receiver);
        let mut can = self.peripherals.configurator();
        configure(&mut can, config.bitrate);
        // The FDCAN pins are configured, so the transceiver may leave
        // standby before FDCAN1 leaves init.
        if config.mode.connects_to_bus() {
            self.transceiver.wake();
        }
        let (mut can_tx, mut can_rx, _) = can.start(operating_mode(config.mode)).split();
        info!("Bus started: {}, {}", config.bitrate, config.mode);

        let mut in_flight = None;
        let mut discarded = 0u32;
        let transmit = async {
            if config.mode.transmits() {
                transmit(&mut can_tx, frame_receiver, &mut in_flight).await
            } else {
                discard(frame_receiver, &mut discarded).await
            }
        };
        let next = match select3(
            commands.wait(),
            receive(&mut can_rx, event_sender, overrun),
            transmit,
        )
        .await
        {
            Either3::First(command) => command,
            Either3::Second(never) | Either3::Third(never) => match never {},
        };

        // Standby first, so the bus never sees the TX pin float when
        // dropping the controller disconnects it.
        self.transceiver.standby();
        drop(can_tx);
        drop(can_rx);

        discarded += u32::from(in_flight.is_some());
        if discarded > 0 {
            info!("Discarded {} frames from USB", discarded);
        }
        next
    }

    /// Keeps the bus stopped until the next command, and returns it. Frames
    /// from the host are discarded and counted, and an overrun still pending
    /// from before the stop is reported once there is room.
    async fn stopped(&mut self, overrun: &mut OverrunCounter) -> BusCommand {
        self.transceiver.standby();
        info!("Bus stopped");

        let mut discarded = 0u32;
        let next = match select3(
            self.commands.wait(),
            discard(self.frame_receiver, &mut discarded),
            report_overrun(self.event_sender, overrun),
        )
        .await
        {
            Either3::First(command) => command,
            Either3::Second(never) | Either3::Third(never) => match never {},
        };

        if discarded > 0 {
            info!(
                "Discarded {} frames from USB while the bus was stopped",
                discarded
            );
        }
        next
    }
}

#[embassy_executor::task]
async fn can_task(mut bus: CanBus) {
    let mut command = BusCommand::POWER_ON;
    let mut overrun = OverrunCounter::new();

    loop {
        debug!("Bus command: {}", command);
        bus.clear(command, &mut overrun);

        command = if command.run {
            bus.run(command.config, &mut overrun).await
        } else {
            bus.stopped(&mut overrun).await
        };
    }
}

/// Sets up a fresh FDCAN1 for `bitrate`.
///
/// From the 48 MHz FDCAN clock, embassy's timing calculation gives 16 time
/// quanta per bit at 125, 250 and 500 kbit/s (prescaler 24, 12 and 6,
/// segments 13 and 2) and 8 at 1 Mbit/s (prescaler 6, segments 6 and 1).
/// All four sample at 87.5 %, with a synchronisation jump width of one time
/// quantum. It finds a solution for each [`Bitrate`], so its unwrap cannot
/// fire.
fn configure(can: &mut CanConfigurator<'_>, bitrate: Bitrate) {
    // Every standard and every extended identifier goes to RX FIFO 0.
    // Remote frames are rejected by the controller: the protocol's Frame
    // cannot say a frame is remote, so passing one on would hand the host a
    // data frame that was never sent.
    let properties = can.properties();
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
    // The TX buffers send in the order they were written. In this mode the
    // driver's write() waits for a free buffer instead of evicting a queued
    // frame of lower priority.
    let config = can
        .config()
        .set_global_filter(global_filter)
        .set_tx_buffer_mode(TxBufferMode::Fifo);
    can.set_config(config);
    can.set_bitrate(bitrate.bits_per_second());
}

/// The FDCAN1 mode for each of the protocol's modes. Listen-only is bus
/// monitoring: the controller receives, but never sends a dominant bit, so
/// it neither transmits nor acknowledges.
fn operating_mode(mode: Mode) -> OperatingMode {
    match mode {
        Mode::Normal => OperatingMode::NormalOperationMode,
        Mode::ListenOnly => OperatingMode::BusMonitoringMode,
        Mode::InternalLoopback => OperatingMode::InternalLoopbackMode,
    }
}

/// Reads the RX FIFO and queues what it reads for the host.
///
/// It never waits for room in the event queue. FDCAN1's RX FIFO holds three
/// frames, and while this task waited it would fill and drop frames without
/// a trace. Events that do not fit are counted by the [`OverrunCounter`]
/// and reported as one `Overrun` as soon as there is room: ahead of the next
/// event, or by itself once the USB side takes an event out of the queue.
async fn receive(
    can_rx: &mut CanRx<'_>,
    event_sender: EventSender,
    overrun: &mut OverrunCounter,
) -> Infallible {
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

/// Queues an overrun left pending when the bus stopped, once the USB side
/// makes room for it.
async fn report_overrun(event_sender: EventSender, overrun: &mut OverrunCounter) -> Infallible {
    loop {
        if !overrun.is_pending() {
            pending::<()>().await;
        }
        poll_fn(|cx| event_sender.poll_ready_to_send(cx)).await;
        if overrun.flush(|event| event_sender.try_send(event)) {
            debug!("Overrun reported");
        }
    }
}

/// Sends the host's frames in the order they arrive. write() waits while
/// all three TX buffers are taken, which in turn fills the frame queue and
/// holds back the USB side.
///
/// The frame being written is kept in `in_flight`, so that the caller can
/// count it as discarded when a command ends the loop during the write.
async fn transmit(
    can_tx: &mut CanTx<'_>,
    frame_receiver: FrameReceiver,
    in_flight: &mut Option<Frame>,
) -> Infallible {
    loop {
        let frame = *in_flight.insert(frame_receiver.receive().await);

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

        *in_flight = None;
    }
}

/// Takes the host's frames and counts them, for when the bus is stopped or
/// listens only. The queue keeps draining, so the USB side never waits.
async fn discard(frame_receiver: FrameReceiver, discarded: &mut u32) -> Infallible {
    loop {
        let frame = frame_receiver.receive().await;
        trace!("Discarded frame: {}", frame);
        *discarded = discarded.saturating_add(1);
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
