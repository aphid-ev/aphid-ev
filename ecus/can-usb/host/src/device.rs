use crate::{Error, Result};
use can_usb_protocol::{
    BULK_IN_EP, BULK_OUT_EP, BusEvent, Config, ControlRequest, DeviceInfo, Frame, INTERFACE,
    MAX_PACKET_SIZE, PRODUCT_ID, PROTOCOL_VERSION, Packer, VENDOR_ID, unpack,
};
use nusb::{
    Endpoint, Interface, MaybeFuture,
    transfer::{Bulk, ControlIn, ControlOut, ControlType, In, Out, Recipient},
};
use std::collections::VecDeque;
use std::time::Duration;

/// How long a control request may take. The interface answers each one
/// without waiting for the bus.
const CONTROL_TIMEOUT: Duration = Duration::from_secs(1);

pub struct UsbCanDevice {
    interface: Interface,
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
    pub fn find() -> Option<nusb::DeviceInfo> {
        nusb::list_devices()
            .wait()
            .ok()?
            .find(|d| d.vendor_id() == VENDOR_ID && d.product_id() == PRODUCT_ID)
    }

    /// Opens the interface, checks that it speaks this protocol version and
    /// starts the bus with the config it has, which is 500 kbit/s in normal
    /// mode unless a program changed it since the interface was plugged in.
    ///
    /// Starting the bus clears the interface's queues, so the events that
    /// piled up while no program read them are gone. At most one packet of
    /// them, up to three events, may still arrive first: the one the
    /// interface had already handed to its USB controller.
    ///
    /// Fails with [`Error::ProtocolMismatch`] when the interface reports
    /// another protocol version, before anything else is sent to it.
    pub fn open(device_info: nusb::DeviceInfo) -> Result<Self> {
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

        let device = Self {
            interface,
            ep_out,
            ep_in,
            events: VecDeque::new(),
            pending_error: None,
        };

        let info = device
            .interface
            .control_in(control_in(ControlRequest::GetDeviceInfo), CONTROL_TIMEOUT)
            .wait()?;
        decode_device_info(&info)?;

        device
            .interface
            .control_out(control_out(ControlRequest::StartBus, &[]), CONTROL_TIMEOUT)
            .wait()?;

        Ok(device)
    }

    /// Asks the interface for its protocol version, firmware version and
    /// serial number.
    pub async fn device_info(&self) -> Result<DeviceInfo> {
        let info = self
            .interface
            .control_in(control_in(ControlRequest::GetDeviceInfo), CONTROL_TIMEOUT)
            .await?;

        decode_device_info(&info)
    }

    /// Sets the bit rate and mode. A running bus restarts with them, a
    /// stopped one keeps them for the next [`start`](Self::start). Either way
    /// the interface clears its queues, and events not yet returned by
    /// [`receive`](Self::receive) are dropped here too, so what comes next is
    /// from the new config. Frames sent but not yet on the bus are discarded.
    pub async fn set_config(&mut self, config: Config) -> Result<()> {
        let (payload, len) = config.encode();

        self.control_out(ControlRequest::SetConfig, &payload[..len])
            .await?;
        self.clear();

        Ok(())
    }

    /// Starts the bus with the current config, or restarts it if it runs,
    /// and clears the queues like [`set_config`](Self::set_config).
    /// [`open`](Self::open) already does this.
    pub async fn start(&mut self) -> Result<()> {
        self.control_out(ControlRequest::StartBus, &[]).await?;
        self.clear();

        Ok(())
    }

    /// Stops the bus: the transceiver goes to standby, and frames sent until
    /// the next [`start`](Self::start) are discarded. Events already received
    /// can still be read.
    pub async fn stop(&self) -> Result<()> {
        self.control_out(ControlRequest::StopBus, &[]).await
    }

    async fn control_out(&self, request: ControlRequest, data: &[u8]) -> Result<()> {
        self.interface
            .control_out(control_out(request, data), CONTROL_TIMEOUT)
            .await?;

        Ok(())
    }

    fn clear(&mut self) {
        self.events.clear();
        self.pending_error = None;
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

            // Every IN packet is shorter than MAX_PACKET_SIZE (see
            // BULK_IN_PACKET_LIMIT), so it ends its transfer: this read
            // completes with exactly one packet as soon as the device sends
            // it, and a larger buffer would not hold more.
            let buffer = self.ep_in.allocate(MAX_PACKET_SIZE);

            self.ep_in.submit(buffer);
            let packet = self.ep_in.next_complete().await.into_result()?;

            self.pending_error = unpack_into(&packet, &mut self.events);
        }
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
    use can_usb_protocol::{BusError, FirmwareVersion, PayloadError};

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

    fn device_info() -> DeviceInfo {
        DeviceInfo {
            protocol_version: PROTOCOL_VERSION,
            firmware_version: FirmwareVersion {
                major: 1,
                minor: 2,
                patch: 3,
            },
            serial: [0xab; DeviceInfo::SERIAL_LEN],
        }
    }

    fn encoded(info: &DeviceInfo) -> Vec<u8> {
        let mut buf = [0; MAX_PACKET_SIZE];
        let len = info.encode(&mut buf).unwrap();
        buf[..len].to_vec()
    }

    #[test]
    fn device_info_of_this_version_decodes() {
        assert_eq!(
            decode_device_info(&encoded(&device_info())).unwrap(),
            device_info()
        );
    }

    #[test]
    fn another_protocol_version_is_a_mismatch() {
        let newer = DeviceInfo {
            protocol_version: PROTOCOL_VERSION + 1,
            ..device_info()
        };

        assert!(matches!(
            decode_device_info(&encoded(&newer)),
            Err(Error::ProtocolMismatch { device, host })
                if device == PROTOCOL_VERSION + 1 && host == PROTOCOL_VERSION
        ));
    }

    /// A newer device may lay out the rest differently; the version alone
    /// decides.
    #[test]
    fn a_mismatch_is_reported_whatever_follows_the_version() {
        let mut bytes = vec![(PROTOCOL_VERSION + 1) as u8];
        bytes.extend_from_slice(&[0xff; 40]);

        assert!(matches!(
            decode_device_info(&bytes),
            Err(Error::ProtocolMismatch { .. })
        ));
    }

    #[test]
    fn a_truncated_device_info_is_a_payload_error() {
        let bytes = encoded(&device_info());

        assert!(matches!(
            decode_device_info(&bytes[..bytes.len() - 1]),
            Err(Error::Payload(PayloadError::Malformed))
        ));
        assert!(matches!(
            decode_device_info(&[]),
            Err(Error::Payload(PayloadError::Malformed))
        ));
    }

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
