use crate::transport::{NusbTransport, TransferError, Transport};
use crate::{Error, Result};
use can_usb_protocol::{
    BusEvent, Config, ControlRequest, DeviceInfo, Frame, INTERFACE, PRODUCT_ID, PROTOCOL_VERSION,
    PackError, Packer, VENDOR_ID, unpack,
};
use nusb::MaybeFuture;
use std::collections::VecDeque;
use std::future::{Future, poll_fn};
use std::pin::pin;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};
use std::thread::{self, Thread};

/// Bulk IN reads kept queued at all times. Each one takes one packet, so the
/// host controller can collect a packet in every USB frame while the
/// program handles the ones before it.
pub const IN_TRANSFERS: usize = 8;

/// Bulk OUT packets in flight at most. A [`send`](UsbCanDevice::send) that
/// needs another one waits until the interface takes one of these.
pub const OUT_TRANSFERS: usize = 4;

/// Events the library holds for [`receive`](UsbCanDevice::receive). When
/// it holds this many, it stops collecting IN packets until the program
/// reads, so the reads stay queued on the computer, the interface's own
/// queue fills, and the interface reports what it loses with
/// [`BusEvent::Overrun`]. The library itself never drops an event.
pub const EVENT_QUEUE_LIMIT: usize = 1024;

/// One CAN/USB interface.
///
/// Frames given to [`send`](Self::send) are packed into bulk OUT packets,
/// with up to [`OUT_TRANSFERS`] packets in flight. A packet goes out when the
/// next frame does not fit in it, at [`flush`](Self::flush), and when the
/// program waits in [`receive`](Self::receive) or calls
/// [`try_receive`](Self::try_receive). [`IN_TRANSFERS`] reads stay queued on
/// bulk IN, and the events they bring wait for [`receive`](Self::receive)
/// in the order the interface sent them.
///
/// [`send`](Self::send), [`flush`](Self::flush) and
/// [`receive`](Self::receive) are cancel-safe, so they can run under a
/// timeout: a dropped `send` has not taken its frame, and a dropped
/// `receive` has not lost an event.
pub struct UsbCanDevice<T: Transport = NusbTransport> {
    transport: T,
    /// Events read from the interface but not yet returned by
    /// [`receive`](Self::receive), and the errors among them in their place.
    events: VecDeque<Result<BusEvent>>,
    /// The OUT packet being filled.
    packer: Packer,
    /// Frames in `packer`.
    packed: usize,
    /// Frames in each OUT transfer in flight, oldest first.
    in_flight: VecDeque<usize>,
    /// Failed OUT transfers not yet reported.
    send_error: Option<Error>,
    /// The interface is gone; every call fails.
    disconnected: bool,
    /// The oldest IN and OUT transfers that `reset` cancelled and nobody
    /// has collected yet, because the reset was dropped. Their completions
    /// are discarded, whatever their status.
    discard_in: usize,
    discard_out: usize,
}

impl UsbCanDevice<NusbTransport> {
    pub fn find() -> Option<nusb::DeviceInfo> {
        nusb::list_devices()
            .wait()
            .ok()?
            .find(|d| d.vendor_id() == VENDOR_ID && d.product_id() == PRODUCT_ID)
    }

    /// Opens the interface and starts its bus at 500 kbit/s in normal mode,
    /// [`Config::DEFAULT`], whatever an earlier program configured. See
    /// [`connect`](Self::connect).
    pub fn open(device_info: nusb::DeviceInfo) -> Result<Self> {
        Self::open_with_config(device_info, Config::DEFAULT)
    }

    /// Opens the interface and starts its bus with `config`. See
    /// [`connect`](Self::connect).
    pub fn open_with_config(device_info: nusb::DeviceInfo, config: Config) -> Result<Self> {
        let device = device_info
            .open()
            .wait()
            .map_err(Error::CouldNotOpenDevice)?;

        let interface = device
            .claim_interface(INTERFACE)
            .wait()
            .map_err(Error::CouldNotOpenDevice)?;

        let transport = NusbTransport::new(interface).map_err(Error::CouldNotOpenDevice)?;

        Self::open_transport(transport, config)
    }
}

impl<T: Transport> UsbCanDevice<T> {
    /// [`connect`](Self::connect), waiting on the calling thread.
    pub fn open_transport(transport: T, config: Config) -> Result<Self> {
        block_on(Self::connect(transport, config))
    }

    /// Checks that the interface speaks this protocol version, sets `config`
    /// and starts the bus, which also restarts a bus an earlier program left
    /// running or stopped.
    ///
    /// Starting the bus clears the interface's queues, so the events that
    /// piled up while no program read them are gone. At most one IN packet
    /// of them may still arrive first: the one the interface had already
    /// handed to its USB controller.
    ///
    /// Fails with [`Error::ProtocolMismatch`] when the interface reports
    /// another protocol version, before anything else is sent to it.
    pub async fn connect(transport: T, config: Config) -> Result<Self> {
        let mut device = Self {
            transport,
            events: VecDeque::new(),
            packer: Packer::new(),
            packed: 0,
            in_flight: VecDeque::new(),
            send_error: None,
            disconnected: false,
            discard_in: 0,
            discard_out: 0,
        };

        device.device_info().await?;

        let (payload, len) = config.encode();
        device
            .reset(&[
                (ControlRequest::SetConfig, &payload[..len]),
                (ControlRequest::StartBus, &[]),
            ])
            .await?;

        Ok(device)
    }

    /// Asks the interface for its protocol version, firmware version and
    /// serial number.
    pub async fn device_info(&self) -> Result<DeviceInfo> {
        self.connected()?;

        let info = self
            .transport
            .control_in(ControlRequest::GetDeviceInfo)
            .await
            .map_err(control_error)?;

        decode_device_info(&info)
    }

    /// Sets the bit rate and mode. A running bus restarts with them, a
    /// stopped one keeps them for the next [`start`](Self::start).
    ///
    /// Everything from before the call is discarded on both sides, so what
    /// [`receive`](Self::receive) returns next is from the new config:
    ///
    /// - Frames sent and not yet on the bus. The library cancels its OUT
    ///   transfers and waits until each one has finished before it sends the
    ///   request, and the interface clears its transmit queue. A frame in a
    ///   packet the interface had already taken but not yet read can still
    ///   go out under the new config.
    /// - Events not yet returned by [`receive`](Self::receive). The library
    ///   cancels its IN reads before the request and throws away what they
    ///   brought, and the interface clears its receive queue. At most one
    ///   IN packet can still arrive from before: the one in the interface's
    ///   IN endpoint buffer.
    ///
    /// Send failures not yet reported are discarded with the frames.
    ///
    /// Dropped before it finishes, as by a timeout, the config may or may
    /// not have reached the interface. The transfers it cancelled are
    /// still discarded when they come back, with whatever they brought,
    /// and later calls work as usual.
    pub async fn set_config(&mut self, config: Config) -> Result<()> {
        let (payload, len) = config.encode();

        self.reset(&[(ControlRequest::SetConfig, &payload[..len])])
            .await
    }

    /// Starts the bus with the current config, or restarts it if it runs,
    /// and discards what came before like [`set_config`](Self::set_config).
    /// [`connect`](Self::connect) already does this.
    pub async fn start(&mut self) -> Result<()> {
        self.reset(&[(ControlRequest::StartBus, &[])]).await
    }

    /// Stops the bus: the transceiver goes to standby, and frames sent until
    /// the next [`start`](Self::start) are discarded. Events already received
    /// can still be read. Unlike [`start`](Self::start), this does not wait
    /// for the OUT transfers, so it also stops a bus where no node
    /// acknowledges and every send waits.
    pub async fn stop(&self) -> Result<()> {
        self.connected()?;

        self.transport
            .control_out(ControlRequest::StopBus, &[])
            .await
            .map_err(control_error)
    }

    /// Queues `frame` to be sent, after every frame queued before it.
    ///
    /// Waits only when the packet being filled is full and
    /// [`OUT_TRANSFERS`] packets are already in flight, which happens while
    /// the interface's transmit queue is full. On a bus where no node
    /// acknowledges, that wait lasts until the program stops the bus; bound
    /// it with a timeout.
    ///
    /// The frame is on its way once this returns `Ok`. If its transfer fails
    /// later, the next `send`, [`send_all`](Self::send_all) or
    /// [`flush`](Self::flush) returns [`Error::SendFailed`] and does not
    /// take its own frame. An `Err` always means `frame` was not taken.
    pub async fn send(&mut self, frame: &Frame) -> Result<()> {
        self.connected()?;
        self.take_send_error()?;

        match self.packer.push(frame) {
            Ok(()) => {}
            Err(PackError::Full) => {
                poll_fn(|cx| {
                    self.pump(cx, false);

                    if self.disconnected || self.transport.pending_out() < OUT_TRANSFERS {
                        Poll::Ready(())
                    } else {
                        Poll::Pending
                    }
                })
                .await;

                self.connected()?;
                self.take_send_error()?;
                self.submit_packet();
                self.packer.push(frame)?;
            }
            Err(error) => return Err(error.into()),
        }

        self.packed += 1;

        // Collect IN packets too, for a program that sends for a while
        // before it reads.
        self.pump(&mut Context::from_waker(Waker::noop()), false);

        Ok(())
    }

    /// Sends `frames` in order like [`send`](Self::send), then sends the
    /// last packet without waiting for it if fewer than [`OUT_TRANSFERS`]
    /// are in flight. Otherwise the last packet goes out at the next
    /// [`flush`](Self::flush), [`receive`](Self::receive) or full packet.
    /// Stops at the first frame that is not taken and returns its error;
    /// the frames before it were taken.
    pub async fn send_all(&mut self, frames: &[Frame]) -> Result<()> {
        for frame in frames {
            self.send(frame).await?;
        }

        self.pump(&mut Context::from_waker(Waker::noop()), true);

        Ok(())
    }

    /// Sends the packet being filled and waits until the interface has taken
    /// every packet in flight, then reports any that failed.
    pub async fn flush(&mut self) -> Result<()> {
        self.connected()?;

        poll_fn(|cx| {
            self.pump(cx, true);

            if self.disconnected || (self.packed == 0 && self.transport.pending_out() == 0) {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        })
        .await;

        self.connected()?;
        self.take_send_error()
    }

    /// The next event from the device, in the order it sent them.
    ///
    /// Sends the packet being filled first, so a program that sends one
    /// request and waits for the answer does not need to call
    /// [`flush`](Self::flush).
    ///
    /// A packet whose tail does not decode yields its whole events first,
    /// then [`Error::Unpack`]. The events after it follow. After the
    /// interface is unplugged, the events received before it come first,
    /// then [`Error::Disconnected`].
    pub async fn receive(&mut self) -> Result<BusEvent> {
        poll_fn(|cx| {
            self.pump(cx, true);

            match self.next_event() {
                Some(event) => Poll::Ready(event),
                None => Poll::Pending,
            }
        })
        .await
    }

    /// The next event if one has arrived, without waiting. Sends the packet
    /// being filled like [`receive`](Self::receive).
    pub fn try_receive(&mut self) -> Option<Result<BusEvent>> {
        self.pump(&mut Context::from_waker(Waker::noop()), true);
        self.next_event()
    }

    fn next_event(&mut self) -> Option<Result<BusEvent>> {
        self.events
            .pop_front()
            .or_else(|| self.disconnected.then_some(Err(Error::Disconnected)))
    }

    fn connected(&self) -> Result<()> {
        if self.disconnected {
            Err(Error::Disconnected)
        } else {
            Ok(())
        }
    }

    fn take_send_error(&mut self) -> Result<()> {
        self.send_error.take().map_or(Ok(()), Err)
    }

    /// Collects finished transfers in both directions without waiting:
    /// records failed OUT transfers, sends the packet being filled if
    /// `flush` and there is room, and moves IN packets into `events` while
    /// it has room. Registers `cx` with every queue it found pending.
    fn pump(&mut self, cx: &mut Context<'_>, flush: bool) {
        // A set_config or start dropped before it finished leaves its reads
        // cancelled and not requeued.
        self.fill_in();
        self.reap_out(cx);

        if flush && self.packed > 0 && self.transport.pending_out() < OUT_TRANSFERS {
            self.submit_packet();
            self.reap_out(cx);
        }

        // At most one round of the queued reads per call: a read that fails
        // as soon as it is submitted would otherwise keep this loop going.
        for _ in 0..IN_TRANSFERS {
            if self.events.len() >= EVENT_QUEUE_LIMIT || self.transport.pending_in() == 0 {
                return;
            }

            match self.transport.poll_in(cx) {
                Poll::Ready(packet) => self.take_packet(packet),
                Poll::Pending => return,
            }
        }

        // More may be ready; the caller polls again instead of waiting.
        cx.waker().wake_by_ref();
    }

    fn reap_out(&mut self, cx: &mut Context<'_>) {
        while self.transport.pending_out() > 0 {
            match self.transport.poll_out(cx) {
                Poll::Ready(result) => {
                    let frames = self.in_flight.pop_front().unwrap_or(0);

                    if self.discard_out > 0 {
                        self.discard_out -= 1;
                        self.disconnected |= result == Err(TransferError::Disconnected);
                    } else if let Err(error) = result {
                        self.send_failed(frames, error);
                    }
                }
                Poll::Pending => break,
            }
        }
    }

    fn send_failed(&mut self, lost: usize, error: TransferError) {
        if error == TransferError::Disconnected {
            self.disconnected = true;
            return;
        }

        match &mut self.send_error {
            Some(Error::SendFailed { frames, .. }) => *frames += lost,
            _ => {
                self.send_error = Some(Error::SendFailed {
                    frames: lost,
                    source: error,
                })
            }
        }
    }

    fn submit_packet(&mut self) {
        self.transport.submit_out(self.packer.as_bytes());
        self.in_flight.push_back(self.packed);
        self.packer.clear();
        self.packed = 0;
    }

    /// Queues the events of a finished IN read and starts the next read.
    fn take_packet(&mut self, packet: std::result::Result<Vec<u8>, TransferError>) {
        match packet {
            Err(TransferError::Disconnected) => self.disconnected = true,
            // A read a dropped reset cancelled: what it brought is from
            // before the reset.
            _ if self.discard_in > 0 => self.discard_in -= 1,
            Ok(packet) => unpack_into(&packet, &mut self.events),
            Err(error) => self.events.push_back(Err(error.into())),
        }

        self.fill_in();
    }

    fn fill_in(&mut self) {
        while !self.disconnected && self.transport.pending_in() < IN_TRANSFERS {
            self.transport.submit_in();
        }
    }

    /// Discards every transfer in flight and every event held, sends
    /// `requests`, and starts reading again.
    async fn reset(&mut self, requests: &[(ControlRequest, &[u8])]) -> Result<()> {
        self.connected()?;

        // Cleared first, so that a reset dropped from here on has
        // discarded them too.
        self.packer.clear();
        self.packed = 0;
        self.send_error = None;
        self.events.clear();

        self.transport.cancel_out();
        self.transport.cancel_in();
        self.discard_out = self.transport.pending_out();
        self.discard_in = self.transport.pending_in();
        poll_fn(|cx| self.poll_drained(cx)).await;
        self.connected()?;

        let mut result = Ok(());
        for (request, data) in requests {
            if let Err(error) = self.transport.control_out(*request, data).await {
                self.disconnected |= error == TransferError::Disconnected;
                result = Err(control_error(error));
                break;
            }
        }

        self.fill_in();

        result
    }

    /// Ready once every cancelled transfer has come back, whatever its
    /// result.
    fn poll_drained(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        while self.discard_out > 0 {
            match self.transport.poll_out(cx) {
                Poll::Ready(result) => {
                    self.discard_out -= 1;
                    self.in_flight.pop_front();
                    self.disconnected |= result == Err(TransferError::Disconnected);
                }
                Poll::Pending => return Poll::Pending,
            }
        }

        while self.discard_in > 0 {
            match self.transport.poll_in(cx) {
                Poll::Ready(result) => {
                    self.discard_in -= 1;
                    self.disconnected |= result == Err(TransferError::Disconnected);
                }
                Poll::Pending => return Poll::Pending,
            }
        }

        Poll::Ready(())
    }
}

fn control_error(error: TransferError) -> Error {
    match error {
        TransferError::Disconnected => Error::Disconnected,
        error => Error::Transfer(error),
    }
}

/// Reads a device info answer, refusing one of another protocol version
/// before decoding the rest of it.
fn decode_device_info(bytes: &[u8]) -> Result<DeviceInfo> {
    let device = DeviceInfo::protocol_version_of(bytes)?;

    if device != PROTOCOL_VERSION {
        return Err(Error::ProtocolMismatch {
            device,
            host: PROTOCOL_VERSION,
        });
    }

    Ok(DeviceInfo::decode(bytes)?)
}

/// Appends every event in `packet` to `events`, then the error for a
/// malformed tail if there is one.
fn unpack_into(packet: &[u8], events: &mut VecDeque<Result<BusEvent>>) {
    for item in unpack(packet) {
        let failed = item.is_err();
        events.push_back(item.map_err(Error::from));

        if failed {
            break;
        }
    }
}

/// Runs `future` on the calling thread until it finishes.
fn block_on<F: Future>(future: F) -> F::Output {
    struct Unpark(Thread);

    impl Wake for Unpark {
        fn wake(self: Arc<Self>) {
            self.0.unpark();
        }
    }

    let waker = Waker::from(Arc::new(Unpark(thread::current())));
    let mut cx = Context::from_waker(&waker);
    let mut future = pin!(future);

    loop {
        if let Poll::Ready(output) = future.as_mut().poll(&mut cx) {
            return output;
        }

        thread::park();
    }
}

#[cfg(test)]
mod tests;
