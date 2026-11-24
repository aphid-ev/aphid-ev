//! The USB pipes [`UsbCanDevice`](crate::UsbCanDevice) talks through, so
//! the library runs against nusb on a computer and against a fake device
//! in tests.

use can_usb_protocol::{ControlRequest, INTERFACE, MAX_PACKET_SIZE};
use nusb::{
    Endpoint, Interface,
    transfer::{Buffer, Bulk, ControlIn, ControlOut, ControlType, In, Out, Recipient},
};
use std::future::{Future, IntoFuture};
use std::task::{Context, Poll};
use std::time::Duration;

pub use nusb::transfer::TransferError;

/// How long a control request may take. The interface answers each one
/// without waiting for the bus.
pub const CONTROL_TIMEOUT: Duration = Duration::from_secs(1);

/// The interface's control requests and its two bulk endpoints.
///
/// Bulk transfers are queued: each `submit_*` starts one, and `poll_*`
/// returns them in the order they were submitted, once they complete. Each
/// IN transfer reads one packet of up to [`MAX_PACKET_SIZE`] bytes, and
/// each OUT transfer writes one packet.
pub trait Transport {
    /// Sends a vendor request to the interface and returns its answer.
    fn control_in(
        &self,
        request: ControlRequest,
    ) -> impl Future<Output = Result<Vec<u8>, TransferError>> + Send;

    /// Sends a vendor request with `data` to the interface.
    fn control_out(
        &self,
        request: ControlRequest,
        data: &[u8],
    ) -> impl Future<Output = Result<(), TransferError>> + Send;

    /// Starts a read of one packet from bulk IN.
    fn submit_in(&mut self);

    /// IN transfers submitted and not yet returned by [`poll_in`](Self::poll_in).
    fn pending_in(&self) -> usize;

    /// The oldest IN transfer's packet once it completes. Pending while
    /// none is submitted.
    fn poll_in(&mut self, cx: &mut Context<'_>) -> Poll<Result<Vec<u8>, TransferError>>;

    /// Asks every pending IN transfer to finish now. Each still comes back
    /// from [`poll_in`](Self::poll_in), cancelled or with what it received.
    fn cancel_in(&mut self);

    /// Starts a write of `packet` to bulk OUT.
    fn submit_out(&mut self, packet: &[u8]);

    /// OUT transfers submitted and not yet returned by [`poll_out`](Self::poll_out).
    fn pending_out(&self) -> usize;

    /// The oldest OUT transfer's result once it completes. Pending while
    /// none is submitted.
    fn poll_out(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), TransferError>>;

    /// Asks every pending OUT transfer to finish now, like
    /// [`cancel_in`](Self::cancel_in).
    fn cancel_out(&mut self);
}

/// [`Transport`] over nusb, to a real interface.
pub struct NusbTransport {
    interface: Interface,
    ep_in: Endpoint<Bulk, In>,
    ep_out: Endpoint<Bulk, Out>,
    /// Buffers of completed transfers, reused so that each transfer does not
    /// allocate one (on Linux, allocating is an mmap).
    free_in: Vec<Buffer>,
    free_out: Vec<Buffer>,
}

impl NusbTransport {
    /// Claims the interface and its bulk endpoints.
    pub fn new(interface: Interface) -> Result<Self, nusb::Error> {
        let ep_in = interface.endpoint::<Bulk, In>(can_usb_protocol::BULK_IN_EP)?;
        let ep_out = interface.endpoint::<Bulk, Out>(can_usb_protocol::BULK_OUT_EP)?;

        Ok(Self {
            interface,
            ep_in,
            ep_out,
            free_in: Vec::new(),
            free_out: Vec::new(),
        })
    }
}

impl Transport for NusbTransport {
    fn control_in(
        &self,
        request: ControlRequest,
    ) -> impl Future<Output = Result<Vec<u8>, TransferError>> + Send {
        self.interface
            .control_in(control_in(request), CONTROL_TIMEOUT)
            .into_future()
    }

    fn control_out(
        &self,
        request: ControlRequest,
        data: &[u8],
    ) -> impl Future<Output = Result<(), TransferError>> + Send {
        self.interface
            .control_out(control_out(request, data), CONTROL_TIMEOUT)
            .into_future()
    }

    fn submit_in(&mut self) {
        let mut buffer = self
            .free_in
            .pop()
            .unwrap_or_else(|| self.ep_in.allocate(MAX_PACKET_SIZE));
        buffer.clear();
        buffer.set_requested_len(MAX_PACKET_SIZE);

        self.ep_in.submit(buffer);
    }

    fn pending_in(&self) -> usize {
        self.ep_in.pending()
    }

    fn poll_in(&mut self, cx: &mut Context<'_>) -> Poll<Result<Vec<u8>, TransferError>> {
        // nusb panics when polled with nothing pending.
        if self.ep_in.pending() == 0 {
            return Poll::Pending;
        }

        self.ep_in.poll_next_complete(cx).map(|completion| {
            let packet = completion.buffer[..completion.actual_len].to_vec();
            self.free_in.push(completion.buffer);

            completion.status.map(|()| packet)
        })
    }

    fn cancel_in(&mut self) {
        self.ep_in.cancel_all();
    }

    fn submit_out(&mut self, packet: &[u8]) {
        let mut buffer = self
            .free_out
            .pop()
            .unwrap_or_else(|| self.ep_out.allocate(MAX_PACKET_SIZE));
        buffer.clear();
        buffer.extend_from_slice(packet);

        self.ep_out.submit(buffer);
    }

    fn pending_out(&self) -> usize {
        self.ep_out.pending()
    }

    fn poll_out(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), TransferError>> {
        if self.ep_out.pending() == 0 {
            return Poll::Pending;
        }

        self.ep_out.poll_next_complete(cx).map(|completion| {
            self.free_out.push(completion.buffer);

            completion.status
        })
    }

    fn cancel_out(&mut self) {
        self.ep_out.cancel_all();
    }
}

/// A vendor request to the interface with a data stage from the device.
fn control_in(request: ControlRequest) -> ControlIn {
    ControlIn {
        control_type: ControlType::Vendor,
        recipient: Recipient::Interface,
        request: request.code(),
        value: 0,
        index: INTERFACE.into(),
        length: MAX_PACKET_SIZE as u16,
    }
}

/// A vendor request to the interface with `data` for the device.
fn control_out(request: ControlRequest, data: &[u8]) -> ControlOut<'_> {
    ControlOut {
        control_type: ControlType::Vendor,
        recipient: Recipient::Interface,
        request: request.code(),
        value: 0,
        index: INTERFACE.into(),
        data,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_requests_address_the_interface() {
        let request = control_in(ControlRequest::GetDeviceInfo);

        assert_eq!(request.control_type, ControlType::Vendor);
        assert_eq!(request.recipient, Recipient::Interface);
        assert_eq!(request.request, 0x10);
        assert_eq!(request.value, 0);
        assert_eq!(request.index, u16::from(INTERFACE));
        assert_eq!(usize::from(request.length), MAX_PACKET_SIZE);

        let request = control_out(ControlRequest::SetConfig, &[2, 1]);

        assert_eq!(request.control_type, ControlType::Vendor);
        assert_eq!(request.recipient, Recipient::Interface);
        assert_eq!(request.request, 0x11);
        assert_eq!(request.value, 0);
        assert_eq!(request.index, u16::from(INTERFACE));
        assert_eq!(request.data, [2, 1]);
    }
}
