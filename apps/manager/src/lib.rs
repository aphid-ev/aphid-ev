//! UI-free core of the manager: everything the desktop app and its CLI do
//! on the bus, testable against a fake bus without hardware.

pub mod bus;
pub mod update;
pub mod usb;

#[cfg(test)]
mod fake;
