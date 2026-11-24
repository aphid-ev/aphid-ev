//! USB identity, endpoints and vendor control requests of the interface.
//!
//! The values here are what both ends open and answer by number, so they
//! are fixed: changing one breaks every host library and firmware already
//! installed.

use core::fmt;

/// USB vendor ID of the interface.
pub const VENDOR_ID: u16 = 0xc0de;

/// USB product ID of the interface.
pub const PRODUCT_ID: u16 = 0xf055;

/// Number of the vendor-specific interface that carries the endpoints and
/// receives the control requests.
pub const INTERFACE: u8 = 0;

/// Bulk OUT endpoint: frames from the host to transmit on the bus.
pub const BULK_OUT_EP: u8 = 0x01;

/// Bulk IN endpoint: [`BusEvent`](crate::BusEvent)s from the interface.
pub const BULK_IN_EP: u8 = 0x82;

/// Largest bulk packet in either direction, and the full-speed bulk maximum.
/// Every packet holds a whole number of values; see [`Packer`](crate::Packer).
pub const MAX_PACKET_SIZE: usize = 64;

/// Version of the protocol this crate describes, reported in
/// [`DeviceInfo`](crate::DeviceInfo). Raise it with every change to the
/// encoding of a value or to the meaning of a request, so that a host can
/// refuse a device it would misread.
pub const PROTOCOL_VERSION: u16 = 1;

/// `bRequest` with which Windows asks for the Microsoft OS 2.0 descriptor
/// set. Reserved: no [`ControlRequest`] may use it.
pub const MSOS_VENDOR_CODE: u8 = 0x01;

/// Vendor control requests, sent to [`INTERFACE`] (`bmRequestType` type
/// vendor, recipient interface, `wValue` 0, `wIndex` the interface number).
///
/// The discriminant is the `bRequest` code. Payloads in the data stage are a
/// single postcard value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
pub enum ControlRequest {
    /// Device to host: [`DeviceInfo`](crate::DeviceInfo).
    GetDeviceInfo = 0x10,
    /// Host to device: a [`Config`](crate::Config) for the bus.
    SetConfig = 0x11,
    /// Host to device, no data: connect to the bus with the current config.
    StartBus = 0x12,
    /// Host to device, no data: disconnect from the bus.
    StopBus = 0x13,
}

impl ControlRequest {
    pub const ALL: [Self; 4] = [
        Self::GetDeviceInfo,
        Self::SetConfig,
        Self::StartBus,
        Self::StopBus,
    ];

    /// The `bRequest` code.
    pub const fn code(self) -> u8 {
        self as u8
    }

    /// Whether the data stage runs from the device to the host.
    pub const fn is_device_to_host(self) -> bool {
        matches!(self, Self::GetDeviceInfo)
    }
}

/// A `bRequest` code that is not a [`ControlRequest`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct UnknownRequest(pub u8);

impl fmt::Display for UnknownRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown control request {:#04x}", self.0)
    }
}

impl core::error::Error for UnknownRequest {}

impl TryFrom<u8> for ControlRequest {
    type Error = UnknownRequest;

    fn try_from(code: u8) -> Result<Self, Self::Error> {
        Self::ALL
            .into_iter()
            .find(|request| request.code() == code)
            .ok_or(UnknownRequest(code))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The host opens the device and its endpoints by these numbers.
    #[test]
    fn identity_and_endpoints_are_stable() {
        assert_eq!(VENDOR_ID, 0xc0de);
        assert_eq!(PRODUCT_ID, 0xf055);
        assert_eq!(INTERFACE, 0);
        assert_eq!(BULK_OUT_EP, 0x01);
        assert_eq!(BULK_IN_EP, 0x82);
        assert_eq!(MAX_PACKET_SIZE, 64);
        assert_eq!(MSOS_VENDOR_CODE, 0x01);
    }

    #[test]
    fn endpoint_directions_follow_the_address_bit() {
        assert_eq!(BULK_OUT_EP & 0x80, 0);
        assert_eq!(BULK_IN_EP & 0x80, 0x80);
    }

    #[test]
    fn request_codes_are_stable() {
        assert_eq!(ControlRequest::GetDeviceInfo.code(), 0x10);
        assert_eq!(ControlRequest::SetConfig.code(), 0x11);
        assert_eq!(ControlRequest::StartBus.code(), 0x12);
        assert_eq!(ControlRequest::StopBus.code(), 0x13);
    }

    #[test]
    fn request_codes_round_trip() {
        for request in ControlRequest::ALL {
            assert_eq!(ControlRequest::try_from(request.code()), Ok(request));
        }
    }

    #[test]
    fn unknown_codes_are_rejected() {
        assert_eq!(ControlRequest::try_from(0x14), Err(UnknownRequest(0x14)));
        assert_eq!(ControlRequest::try_from(0xff), Err(UnknownRequest(0xff)));
    }

    #[test]
    fn no_request_takes_the_msos_vendor_code() {
        assert!(ControlRequest::try_from(MSOS_VENDOR_CODE).is_err());
    }

    #[test]
    fn only_device_info_reads_from_the_device() {
        for request in ControlRequest::ALL {
            assert_eq!(
                request.is_device_to_host(),
                request == ControlRequest::GetDeviceInfo
            );
        }
    }
}
