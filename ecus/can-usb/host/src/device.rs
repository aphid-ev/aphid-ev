use crate::{Error, Result};
use can_usb_protocol::{BusEvent, Frame};
use nusb::{
    DeviceInfo, Endpoint, Interface, MaybeFuture,
    transfer::{Bulk, ControlIn, ControlOut, ControlType, In, Out, Recipient},
};
use std::time::Duration;

const VENDOR_ID: u16 = 0xc0de;
const PRODUCT_ID: u16 = 0xf055;

const BULK_OUT_EP: u8 = 0x01;
const BULK_IN_EP: u8 = 0x82;

const CONTROL_TIMEOUT: Duration = Duration::from_millis(500);

pub struct UsbCanDevice {
    interface: Interface,
    ep_out: Endpoint<Bulk, Out>,
    ep_in: Endpoint<Bulk, In>,
}

impl UsbCanDevice {
    pub fn find() -> Option<DeviceInfo> {
        nusb::list_devices()
            .wait()
            .ok()?
            .find(|d| d.vendor_id() == VENDOR_ID && d.product_id() == PRODUCT_ID)
    }

    pub fn open(device_info: DeviceInfo) -> Result<Self> {
        let device = device_info
            .open()
            .wait()
            .map_err(Error::CouldNotOpenDevice)?;

        let interface = device
            .claim_interface(0)
            .wait()
            .map_err(Error::CouldNotOpenDevice)?;

        let ep_out = interface
            .endpoint::<Bulk, Out>(BULK_OUT_EP)
            .map_err(Error::CouldNotOpenDevice)?;
        let ep_in = interface
            .endpoint::<Bulk, In>(BULK_IN_EP)
            .map_err(Error::CouldNotOpenDevice)?;

        Ok(Self {
            interface,
            ep_out,
            ep_in,
        })
    }

    #[allow(dead_code)] // TODO: Maybe use this to start listening later
    pub async fn control_out(&mut self, data: &[u8]) -> Result<()> {
        let data = ControlOut {
            control_type: ControlType::Vendor,
            recipient: Recipient::Interface,
            request: 100,
            value: 200,
            index: 0,
            data,
        };

        self.interface.control_out(data, CONTROL_TIMEOUT).await?;

        Ok(())
    }

    #[allow(dead_code)] // TODO: Maybe use this to start listening later
    pub async fn control_in(&mut self, length: u16) -> Result<Vec<u8>> {
        let data = ControlIn {
            control_type: ControlType::Vendor,
            recipient: Recipient::Interface,
            request: 101,
            value: 201,
            index: 0,
            length,
        };

        Ok(self.interface.control_in(data, CONTROL_TIMEOUT).await?)
    }

    pub async fn send(&mut self, frame: &Frame) -> Result<()> {
        let bytes = postcard::to_allocvec(&frame)?;

        let mut buffer = self.ep_out.allocate(bytes.len());
        buffer.extend_from_slice(&bytes);

        self.ep_out.submit(buffer);
        self.ep_out.next_complete().await.status?;

        Ok(())
    }

    pub async fn receive(&mut self) -> Result<BusEvent> {
        let buffer = self.ep_in.allocate(64);

        self.ep_in.submit(buffer);
        let data = self.ep_in.next_complete().await.into_result()?;

        Ok(postcard::from_bytes(&data)?)
    }
}
