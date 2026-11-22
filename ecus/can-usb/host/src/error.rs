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

    #[error("Failed serialization/deserialization with postcard: {0}")]
    Postcard(#[from] postcard::Error),
}
