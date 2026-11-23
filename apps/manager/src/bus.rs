//! The CAN bus as the core sees it.

use std::future::Future;

pub use boot_protocol::Frame;

/// A failure of the CAN interface itself, not of a node on the bus.
#[derive(Debug, thiserror::Error)]
#[error("CAN interface: {0}")]
pub struct BusError(pub Box<dyn std::error::Error + Send + Sync>);

/// A CAN interface that sends and receives classic frames with standard
/// identifiers. Implementations drop every other kind of frame on receive.
pub trait Bus {
    fn send(&mut self, frame: &Frame) -> impl Future<Output = Result<(), BusError>>;

    /// Waits for the next frame. Callers bound the wait with a timeout, so
    /// this may wait forever on a quiet bus.
    fn receive(&mut self) -> impl Future<Output = Result<Frame, BusError>>;
}
