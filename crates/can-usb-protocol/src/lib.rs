//! The USB protocol between the CAN/USB interface and the host.
//!
//! - [`usb`]: identity, endpoints, packet size, [`PROTOCOL_VERSION`] and the
//!   vendor [`ControlRequest`]s.
//! - Bulk OUT carries [`Frame`]s to transmit; bulk IN carries [`BusEvent`]s.
//!   Each bulk packet is a whole number of postcard values, built with a
//!   [`Packer`] and read with [`unpack`] (see [`packet`]).
//! - Control requests carry [`DeviceInfo`] and [`Config`].
//!
//! The firmware and the host are flashed and installed separately, so the
//! `wire_encoding_is_stable` tests pin the bytes. Change them only to break
//! compatibility on purpose, and raise [`PROTOCOL_VERSION`] when you do.

// Tests link the standard test harness, so std is only excluded otherwise.
#![cfg_attr(not(test), no_std)]

mod bus_error;
mod bus_event;
mod control;
mod frame;
pub mod packet;
pub mod usb;

pub use bus_error::*;
pub use bus_event::*;
pub use control::*;
pub use frame::*;
pub use packet::{PackError, Packer, UnpackError, Unpacker, unpack};
pub use usb::*;
