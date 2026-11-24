//! Host side of the CAN/USB interface: finds the device, configures it over
//! control requests and streams [`can_usb_protocol`] frames and events over
//! its bulk endpoints, with several transfers in flight in each direction.
//!
//! [`UsbCanDevice`] talks through a [`Transport`]: [`NusbTransport`] for a
//! real interface, or a fake device in the tests.

mod device;
mod error;
#[cfg(test)]
mod fake;
mod transport;

pub use device::*;
pub use error::*;
pub use transport::*;
