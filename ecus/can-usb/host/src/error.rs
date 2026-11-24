use nusb::transfer::TransferError;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Failed to open device: {0}")]
    CouldNotOpenDevice(nusb::Error),

    #[error("Could not find any UsbCan devices connected")]
    DeviceNotFound,

    #[error("USB error: {0}")]
    Usb(#[from] nusb::Error),

    #[error("Failed transfer: {0}")]
    Transfer(#[from] TransferError),

    /// The interface was unplugged or reset. Every later call fails with
    /// this too; open the interface again.
    #[error("The CAN/USB interface was disconnected")]
    Disconnected,

    /// Bulk OUT transfers failed, so `frames` frames that
    /// [`send`](crate::UsbCanDevice::send) had accepted never reached the
    /// interface. `source` is the first failure.
    #[error("{frames} frames could not be sent to the interface: {source}")]
    SendFailed {
        frames: usize,
        source: TransferError,
    },

    #[error("Could not encode a frame: {0}")]
    Pack(#[from] can_usb_protocol::PackError),

    #[error("Could not decode a packet from the device: {0}")]
    Unpack(#[from] can_usb_protocol::UnpackError),

    #[error(
        "The device speaks protocol version {device} and this program version {host}; \
         update the one that is older"
    )]
    ProtocolMismatch { device: u16, host: u16 },

    #[error("Bad control request payload: {0}")]
    Payload(#[from] can_usb_protocol::PayloadError),
}
