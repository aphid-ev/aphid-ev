use crate::{Error, Result};
use can_usb_protocol::{
    BULK_IN_EP, BULK_OUT_EP, BusEvent, Frame, INTERFACE, MAX_PACKET_SIZE, PRODUCT_ID, Packer,
    VENDOR_ID, unpack,
};
use nusb::{
    DeviceInfo, Endpoint, MaybeFuture,
    transfer::{Bulk, In, Out},
};
use std::collections::VecDeque;

pub struct UsbCanDevice {
    ep_out: Endpoint<Bulk, Out>,
    ep_in: Endpoint<Bulk, In>,
    /// Events already read from the device but not yet returned by
    /// [`UsbCanDevice::receive`], because one packet carries several.
    events: VecDeque<BusEvent>,
    /// A malformed tail of the last packet, reported after the events before
    /// it.
    pending_error: Option<Error>,
}

impl UsbCanDevice {
    pub fn find() -> Option<DeviceInfo> {
        nusb::list_devices()
            .wait()
            .ok()?
            .find(|d| d.vendor_id() == VENDOR_ID && d.product_id() == PRODUCT_ID)
    }

    pub fn open(device_info: DeviceInfo) -> Result<Self> {
        let device = device_info
            .open()
            .wait()
            .map_err(Error::CouldNotOpenDevice)?;

        let interface = device
            .claim_interface(INTERFACE)
            .wait()
            .map_err(Error::CouldNotOpenDevice)?;

        let ep_out = interface
            .endpoint::<Bulk, Out>(BULK_OUT_EP)
            .map_err(Error::CouldNotOpenDevice)?;
        let ep_in = interface
            .endpoint::<Bulk, In>(BULK_IN_EP)
            .map_err(Error::CouldNotOpenDevice)?;

        Ok(Self {
            ep_out,
            ep_in,
            events: VecDeque::new(),
            pending_error: None,
        })
    }

    /// Sends one frame in a packet of its own.
    pub async fn send(&mut self, frame: &Frame) -> Result<()> {
        let mut packer = Packer::new();
        packer.push(frame)?;

        let mut buffer = self.ep_out.allocate(packer.len());
        buffer.extend_from_slice(packer.as_bytes());

        self.ep_out.submit(buffer);
        self.ep_out.next_complete().await.status?;

        Ok(())
    }

    /// The next event from the device, in the order it sent them.
    ///
    /// A packet whose tail does not decode yields its whole events first,
    /// then [`Error::Unpack`].
    pub async fn receive(&mut self) -> Result<BusEvent> {
        loop {
            if let Some(event) = self.events.pop_front() {
                return Ok(event);
            }

            if let Some(error) = self.pending_error.take() {
                return Err(error);
            }

            let buffer = self.ep_in.allocate(MAX_PACKET_SIZE);

            self.ep_in.submit(buffer);
            let packet = self.ep_in.next_complete().await.into_result()?;

            self.pending_error = unpack_into(&packet, &mut self.events);
        }
    }
}

/// Appends every event in `packet` to `events` and returns the error for a
/// malformed tail, if there is one.
fn unpack_into(packet: &[u8], events: &mut VecDeque<BusEvent>) -> Option<Error> {
    for item in unpack(packet) {
        match item {
            Ok(event) => events.push_back(event),
            Err(error) => return Some(error.into()),
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use can_usb_protocol::BusError;

    fn events() -> [BusEvent; 3] {
        [
            BusEvent::Frame {
                timestamp_us: 1,
                frame: Frame::new(0x123u16, &[1, 2, 3]).unwrap(),
            },
            BusEvent::BusError {
                timestamp_us: 2,
                error: BusError::Crc,
            },
            BusEvent::Overrun {
                timestamp_us: 3,
                lost: 4,
            },
        ]
    }

    fn packet(events: &[BusEvent]) -> Vec<u8> {
        let mut packer = Packer::new();

        for event in events {
            packer.push(event).unwrap();
        }

        packer.as_bytes().to_vec()
    }

    #[test]
    fn every_event_in_a_packet_is_kept_in_order() {
        let mut queue = VecDeque::new();

        assert!(unpack_into(&packet(&events()), &mut queue).is_none());
        assert_eq!(Vec::from(queue), events());
    }

    #[test]
    fn a_zero_length_packet_adds_nothing() {
        let mut queue = VecDeque::new();

        assert!(unpack_into(&[], &mut queue).is_none());
        assert!(queue.is_empty());
    }

    #[test]
    fn a_malformed_tail_keeps_the_events_before_it() {
        let mut bytes = packet(&events());
        bytes.push(0xff);
        let mut queue = VecDeque::new();

        let error = unpack_into(&bytes, &mut queue);

        assert!(matches!(error, Some(Error::Unpack(_))));
        assert_eq!(Vec::from(queue), events());
    }
}
