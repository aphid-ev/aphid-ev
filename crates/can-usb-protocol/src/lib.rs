//! Wire format of the CAN/USB interface's bulk endpoints.
//!
//! Each transfer carries postcard-encoded values: [`Frame`] from host to
//! device, [`BusEvent`] from device to host. The firmware and the host are
//! flashed and installed separately, so the `wire_encoding_is_stable` tests
//! pin the bytes. Change them only to break compatibility on purpose.

// Tests link the standard test harness, so std is only excluded otherwise.
#![cfg_attr(not(test), no_std)]

mod bus_error;
mod bus_event;
mod frame;

pub use bus_error::*;
pub use bus_event::*;
pub use frame::*;
