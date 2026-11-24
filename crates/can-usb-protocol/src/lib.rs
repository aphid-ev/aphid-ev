//! The USB protocol between the CAN/USB interface and the host.
//!
//! - [`usb`]: identity, endpoints, packet size, [`PROTOCOL_VERSION`] and the
//!   vendor [`ControlRequest`]s.
//! - Bulk OUT carries [`Frame`]s to transmit; bulk IN carries [`BusEvent`]s.
//!   Each bulk packet is a whole number of postcard values, built with a
//!   [`Packer`] and read with [`unpack`] (see [`packet`]).
//! - Control requests carry [`DeviceInfo`] and [`Config`]. A [`Controller`]
//!   decides what the interface does with each one (see [`controller`]).
//! - [`Batcher`] builds IN packets from the interface's event queue, and
//!   [`DropTracker`] accounts for events a failed write drops (see [`batch`]).
//! - [`OverrunCounter`] turns received frames the interface could not queue
//!   into one [`BusEvent::Overrun`] (see [`overrun`]).
//! - [`BusMonitor`] turns the controller's status into
//!   [`BusEvent::BusState`] events and decides when to recover from bus-off
//!   (see [`bus_monitor`]).
//! - [`TimestampUnwrapper`] turns the events' wrapping 32-bit timestamps
//!   into a 64-bit time on the host (see [`timestamp`]).
//!
//! The firmware and the host are flashed and installed separately, so the
//! `wire_encoding_is_stable` tests pin the bytes. Change them only to break
//! compatibility on purpose, and raise [`PROTOCOL_VERSION`] when you do.

// Tests link the standard test harness, so std is only excluded otherwise.
#![cfg_attr(not(test), no_std)]

pub mod batch;
mod bus_error;
mod bus_event;
pub mod bus_monitor;
mod control;
pub mod controller;
mod frame;
pub mod overrun;
pub mod packet;
pub mod timestamp;
pub mod usb;

pub use batch::{Batcher, DropLog, DropTracker};
pub use bus_error::*;
pub use bus_event::*;
pub use bus_monitor::{BusMonitor, ControllerStatus, Update};
pub use control::*;
pub use controller::{BusCommand, Controller, Rejection};
pub use frame::*;
pub use overrun::{Forwarded, OverrunCounter};
pub use packet::{PackError, Packer, UnpackError, Unpacker, unpack};
pub use timestamp::{TIMESTAMP_PERIOD_US, TimestampUnwrapper};
pub use usb::*;
