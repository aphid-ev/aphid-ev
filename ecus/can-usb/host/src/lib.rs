//! Host side of the CAN/USB interface: finds the device and exchanges
//! [`can_usb_protocol`] frames with it over its bulk endpoints.

mod device;
mod error;

pub use device::*;
pub use error::*;
