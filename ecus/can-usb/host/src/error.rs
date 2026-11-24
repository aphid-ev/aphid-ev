use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Failed to open device: {0}")]
    CouldNotOpenDevice(nusb::Error),

    #[error("Could not find any UsbCan devices connected")]
    DeviceNotFound,

    #[error("USb error: {0}")]
    Usb(#[from] nusb::Error),

    #[error("Failed transver: {0}")]
    Transfer(#[from] nusb::transfer::TransferError),

    #[error("Could not encode a frame: {0}")]
    Pack(#[from] can_usb_protocol::PackError),

    #[error("Could not decode a packet from the device: {0}")]
    Unpack(#[from] can_usb_protocol::UnpackError),
}
