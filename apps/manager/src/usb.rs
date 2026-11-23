//! [`Bus`] over the CAN/USB interface.

use crate::bus::{Bus, BusError, Frame};
use can_usb_host::UsbCanDevice;
use can_usb_protocol::{BusEvent, FrameId};

pub struct UsbBus {
    device: UsbCanDevice,
}

impl UsbBus {
    /// Opens the first CAN/USB interface found.
    pub fn open() -> Result<Self, BusError> {
        let info =
            UsbCanDevice::find().ok_or_else(|| error(can_usb_host::Error::DeviceNotFound))?;
        let device = UsbCanDevice::open(info).map_err(error)?;

        Ok(Self { device })
    }
}

fn error(error: impl std::error::Error + Send + Sync + 'static) -> BusError {
    BusError(Box::new(error))
}

impl Bus for UsbBus {
    async fn send(&mut self, frame: &Frame) -> Result<(), BusError> {
        let frame = can_usb_protocol::Frame::new(frame.id(), frame.data()).map_err(error)?;

        self.device.send(&frame).await.map_err(error)
    }

    /// Error frames on the bus are not failures of the interface; they are
    /// skipped like extended frames. A node that does not answer shows up as
    /// a timeout in the caller.
    async fn receive(&mut self) -> Result<Frame, BusError> {
        loop {
            if let BusEvent::Frame(frame) = self.device.receive().await.map_err(error)?
                && let FrameId::Standard(id) = frame.id()
                && let Some(frame) = Frame::new(id, frame.data())
            {
                return Ok(frame);
            }
        }
    }
}
