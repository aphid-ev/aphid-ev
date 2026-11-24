//! A fake interface behind [`Transport`], driven by the tests.
//!
//! The test keeps a clone of [`FakeDevice`] to play the interface while a
//! [`UsbCanDevice`](crate::UsbCanDevice) owns another. Transfers complete
//! as soon as the device side allows: an IN read when the device has a
//! packet for it, an OUT write while the device accepts (it NAKs otherwise).

use crate::transport::{TransferError, Transport};
use can_usb_protocol::{ControlRequest, DeviceInfo, FirmwareVersion, MAX_PACKET_SIZE};
use std::collections::VecDeque;
use std::future::{Future, poll_fn};
use std::sync::{Arc, Mutex, MutexGuard};
use std::task::{Context, Poll, Waker};

type Completion<T> = Result<T, TransferError>;

#[derive(Clone)]
pub struct FakeDevice(Arc<Mutex<State>>);

struct State {
    /// The answer to get device info.
    info: Vec<u8>,
    /// Every control request with its data, and the OUT transfers still
    /// pending when it was sent.
    controls: Vec<(ControlRequest, Vec<u8>, usize)>,
    /// Requests the device STALLs.
    stalls: Vec<ControlRequest>,
    /// Control requests wait until this is cleared, like a slow device.
    holding_controls: bool,
    /// Packets the device has to send, oldest first.
    to_host: VecDeque<Completion<Vec<u8>>>,
    /// IN reads waiting for a packet.
    reads: usize,
    /// IN reads finished but not yet collected.
    reads_done: VecDeque<Completion<Vec<u8>>>,
    /// OUT packets waiting for the device to take them.
    writes: VecDeque<Vec<u8>>,
    /// OUT writes finished but not yet collected.
    writes_done: VecDeque<Completion<()>>,
    /// Packets the device took, oldest first.
    received: Vec<Vec<u8>>,
    /// Whether the device takes OUT packets or NAKs them.
    accepting: bool,
    /// The next OUT packet the device would take fails with this instead.
    fail_next_write: Option<TransferError>,
    disconnected: bool,
    waker: Option<Waker>,
}

impl FakeDevice {
    /// A device of this protocol version that accepts every OUT packet.
    pub fn new() -> Self {
        Self::with_info(info(can_usb_protocol::PROTOCOL_VERSION))
    }

    pub fn with_info(info: Vec<u8>) -> Self {
        Self(Arc::new(Mutex::new(State {
            info,
            controls: Vec::new(),
            stalls: Vec::new(),
            holding_controls: false,
            to_host: VecDeque::new(),
            reads: 0,
            reads_done: VecDeque::new(),
            writes: VecDeque::new(),
            writes_done: VecDeque::new(),
            received: Vec::new(),
            accepting: true,
            fail_next_write: None,
            disconnected: false,
            waker: None,
        })))
    }

    fn state(&self) -> MutexGuard<'_, State> {
        self.0.lock().unwrap()
    }

    /// Requests the device has been sent, with their data.
    pub fn controls(&self) -> Vec<(ControlRequest, Vec<u8>)> {
        self.state()
            .controls
            .iter()
            .map(|(request, data, _)| (*request, data.clone()))
            .collect()
    }

    /// For each request, the OUT transfers pending when it was sent.
    pub fn writes_pending_at_controls(&self) -> Vec<usize> {
        self.state().controls.iter().map(|c| c.2).collect()
    }

    /// Makes control requests wait (`true`) or answer at once (`false`).
    pub fn hold_controls(&self, holding: bool) {
        let mut state = self.state();
        state.holding_controls = holding;
        if let Some(waker) = state.waker.take() {
            waker.wake();
        }
    }

    /// Answers `request` once controls are not held, like a control
    /// transfer that ends when the device answers. Dropped before that, the
    /// request never reaches the device.
    fn control_future<T>(
        &self,
        request: ControlRequest,
        data: &[u8],
        answer: impl Fn(&State) -> T + Send,
    ) -> impl Future<Output = Completion<T>> + Send {
        let fake = self.clone();
        let data = data.to_vec();

        poll_fn(move |cx| {
            let mut state = fake.state();

            if state.holding_controls && !state.disconnected {
                state.waker = Some(cx.waker().clone());
                return Poll::Pending;
            }

            Poll::Ready(state.control(request, &data).map(|()| answer(&state)))
        })
    }

    pub fn stall(&self, request: ControlRequest) {
        self.state().stalls.push(request);
    }

    /// The device sends `packet` on bulk IN.
    pub fn send_packet(&self, packet: Vec<u8>) {
        self.complete_read(Ok(packet));
    }

    /// The next IN read fails with `error`.
    pub fn fail_read(&self, error: TransferError) {
        self.complete_read(Err(error));
    }

    fn complete_read(&self, result: Completion<Vec<u8>>) {
        let mut state = self.state();
        state.to_host.push_back(result);
        state.run();
    }

    /// Packets the device still has to send because no read was queued.
    pub fn unsent_packets(&self) -> usize {
        self.state().to_host.len()
    }

    pub fn reads_queued(&self) -> usize {
        let state = self.state();
        state.reads + state.reads_done.len()
    }

    pub fn writes_queued(&self) -> usize {
        let state = self.state();
        state.writes.len() + state.writes_done.len()
    }

    /// OUT packets the device took, oldest first.
    pub fn received(&self) -> Vec<Vec<u8>> {
        self.state().received.clone()
    }

    /// Takes OUT packets (`true`) or NAKs them (`false`).
    pub fn accept(&self, accepting: bool) {
        let mut state = self.state();
        state.accepting = accepting;
        state.run();
    }

    pub fn fail_next_write(&self, error: TransferError) {
        self.state().fail_next_write = Some(error);
    }

    /// Unplugs the device: every pending transfer fails, and so does every
    /// later one.
    pub fn disconnect(&self) {
        let mut state = self.state();
        state.disconnected = true;
        state.run();
    }
}

impl State {
    /// Finishes every transfer the device side now allows, and wakes the
    /// host if any finished.
    fn run(&mut self) {
        let mut progress = false;

        if self.disconnected {
            for _ in 0..self.reads {
                self.reads_done.push_back(Err(TransferError::Disconnected));
            }
            for _ in self.writes.drain(..) {
                self.writes_done.push_back(Err(TransferError::Disconnected));
            }
            progress |= self.reads > 0;
            self.reads = 0;
        }

        while self.reads > 0
            && let Some(packet) = self.to_host.pop_front()
        {
            self.reads -= 1;
            self.reads_done.push_back(packet);
            progress = true;
        }

        while self.accepting
            && let Some(packet) = self.writes.pop_front()
        {
            match self.fail_next_write.take() {
                Some(error) => self.writes_done.push_back(Err(error)),
                None => {
                    self.received.push(packet);
                    self.writes_done.push_back(Ok(()));
                }
            }
            progress = true;
        }

        if (progress || self.disconnected)
            && let Some(waker) = self.waker.take()
        {
            waker.wake();
        }
    }

    fn control(&mut self, request: ControlRequest, data: &[u8]) -> Completion<()> {
        if self.disconnected {
            return Err(TransferError::Disconnected);
        }

        let pending = self.writes.len() + self.writes_done.len();
        self.controls.push((request, data.to_vec(), pending));

        if self.stalls.contains(&request) {
            Err(TransferError::Stall)
        } else {
            Ok(())
        }
    }
}

impl Transport for FakeDevice {
    fn control_in(
        &self,
        request: ControlRequest,
    ) -> impl Future<Output = Completion<Vec<u8>>> + Send {
        self.control_future(request, &[], |state| state.info.clone())
    }

    fn control_out(
        &self,
        request: ControlRequest,
        data: &[u8],
    ) -> impl Future<Output = Completion<()>> + Send {
        self.control_future(request, data, |_| ())
    }

    fn submit_in(&mut self) {
        let mut state = self.state();
        state.reads += 1;
        state.run();
    }

    fn pending_in(&self) -> usize {
        self.reads_queued()
    }

    fn poll_in(&mut self, cx: &mut Context<'_>) -> Poll<Completion<Vec<u8>>> {
        let mut state = self.state();

        match state.reads_done.pop_front() {
            Some(packet) => Poll::Ready(packet.map(|mut packet| {
                packet.truncate(MAX_PACKET_SIZE);
                packet
            })),
            None => {
                state.waker = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }

    fn cancel_in(&mut self) {
        let mut state = self.state();

        for _ in 0..state.reads {
            state.reads_done.push_back(Err(TransferError::Cancelled));
        }
        state.reads = 0;
    }

    fn submit_out(&mut self, packet: &[u8]) {
        assert!(packet.len() <= MAX_PACKET_SIZE, "OUT packet too long");

        let mut state = self.state();
        state.writes.push_back(packet.to_vec());
        state.run();
    }

    fn pending_out(&self) -> usize {
        self.writes_queued()
    }

    fn poll_out(&mut self, cx: &mut Context<'_>) -> Poll<Completion<()>> {
        let mut state = self.state();

        match state.writes_done.pop_front() {
            Some(result) => Poll::Ready(result),
            None => {
                state.waker = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }

    fn cancel_out(&mut self) {
        let mut state = self.state();

        while let Some(_packet) = state.writes.pop_front() {
            state.writes_done.push_back(Err(TransferError::Cancelled));
        }
    }
}

/// A device info answer for `protocol_version`.
pub fn info(protocol_version: u16) -> Vec<u8> {
    let info = DeviceInfo {
        protocol_version,
        firmware_version: FirmwareVersion {
            major: 1,
            minor: 2,
            patch: 3,
        },
        serial: [0xab; DeviceInfo::SERIAL_LEN],
    };
    let mut buf = [0; MAX_PACKET_SIZE];
    let len = info.encode(&mut buf).unwrap();

    buf[..len].to_vec()
}
