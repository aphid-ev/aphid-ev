use crate::{RxReceiver, TxSender};
use can_usb_protocol::Frame;
use defmt::{error, info, unwrap, warn};
use embassy_executor::Spawner;
use embassy_stm32::usb::{Endpoint, In, Out};
use embassy_stm32::{peripherals::USB, usb::Driver};
use embassy_usb::Builder;
use embassy_usb::driver::{Endpoint as _, EndpointAddress, EndpointIn, EndpointOut};
use embassy_usb::msos::{self, windows_version};
use static_cell::StaticCell;

static CONFIG_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
static BOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
static MSOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
static CONTROL_BUF: StaticCell<[u8; 64]> = StaticCell::new();

/// Identifies this interface to Windows. Generated once and fixed from then
/// on: Windows caches the driver binding against this value, so changing it
/// orphans the binding on every machine that has already seen the device.
const DEVICE_INTERFACE_GUID: &str = "{19F2234B-041A-4811-B321-E288246821A7}";

/// bRequest that Windows uses to ask for the descriptor set. Any value will
/// do while nothing else on this device answers vendor requests.
const MSOS_VENDOR_CODE: u8 = 0x01;

pub struct UsbDevice {
    usb_driver: Driver<'static, USB>,
    rx_receiver: RxReceiver,
    tx_sender: TxSender,
}

impl UsbDevice {
    pub fn new(
        usb_driver: Driver<'static, USB>,
        rx_receiver: RxReceiver,
        tx_sender: TxSender,
    ) -> Self {
        Self {
            usb_driver,
            rx_receiver,
            tx_sender,
        }
    }

    pub fn start(self, spawner: Spawner) {
        let mut config = embassy_usb::Config::new(0xc0de, 0xf055);
        config.manufacturer = Some("Aphid-ev");
        config.product = Some("can-interface");
        config.serial_number = Some("1");
        config.max_power = 100;
        config.max_packet_size_0 = 64;

        let config_descriptor = CONFIG_DESCRIPTOR.init([0; 256]);
        let bos_descriptor = BOS_DESCRIPTOR.init([0; 256]);
        let msos_descriptor = MSOS_DESCRIPTOR.init([0; 256]);
        let control_buf = CONTROL_BUF.init([0; 64]);

        let mut builder = Builder::new(
            self.usb_driver,
            config,
            config_descriptor,
            bos_descriptor,
            msos_descriptor,
            control_buf,
        );

        // Microsoft OS 2.0 descriptors, so that Windows binds WinUSB to this
        // interface by itself. Without them the device still enumerates, but
        // it has no driver and the host application cannot open it until
        // someone binds WinUSB by hand with a tool like Zadig. macOS and
        // Linux never ask for these.
        builder.msos_descriptor(windows_version::WIN8_1, MSOS_VENDOR_CODE);

        // A vendor-specific function (class 0xFF) carrying the two bulk
        // endpoints. The addresses are pinned because the host application
        // opens bulk OUT 0x01 and bulk IN 0x82 by address.
        let mut function = builder.function(0xFF, 0, 0);
        function.msos_feature(msos::CompatibleIdFeatureDescriptor::new("WINUSB", ""));
        function.msos_feature(msos::RegistryPropertyFeatureDescriptor::new(
            "DeviceInterfaceGUIDs",
            msos::PropertyData::RegMultiSz(&[DEVICE_INTERFACE_GUID]),
        ));
        let mut interface = function.interface();
        let mut alt = interface.alt_setting(0xFF, 0, 0, None);
        let read_ep = alt.endpoint_bulk_out(Some(EndpointAddress::from(0x01)), 64);
        let write_ep = alt.endpoint_bulk_in(Some(EndpointAddress::from(0x82)), 64);
        drop(function);

        let usb = builder.build();

        // As in CanBus::start, spawn failure is a double-start programming
        // error the device can't recover from, so panic via defmt.
        spawner.spawn(unwrap!(read_task(read_ep, self.tx_sender)));
        spawner.spawn(unwrap!(write_task(write_ep, self.rx_receiver)));
        spawner.spawn(unwrap!(usb_task(usb)));
    }
}

#[embassy_executor::task]
async fn read_task(mut read_ep: Endpoint<'static, USB, Out>, tx_sender: TxSender) {
    let mut buf = [0u8; 64];

    loop {
        read_ep.wait_enabled().await;
        info!("USB connected");

        while let Ok(len) = read_ep.read(&mut buf).await {
            // Frames are variable length, so a transfer can carry several of
            // them back to back. Keep decoding until the packet is used up:
            // from_bytes would return only the first and discard the rest.
            let mut remaining = &buf[..len];

            while !remaining.is_empty() {
                match postcard::take_from_bytes::<Frame>(remaining) {
                    Ok((frame, rest)) => {
                        remaining = rest;
                        tx_sender.send(frame).await;
                    }
                    Err(error) => {
                        error!("Failed to deserialize postcard data: {}", error);
                        break;
                    }
                }
            }
        }

        info!("USB disconnected");
    }
}

#[embassy_executor::task]
async fn write_task(mut write_ep: Endpoint<'static, USB, In>, rx_receiver: RxReceiver) {
    let mut buf = [0u8; 64];

    loop {
        let event = rx_receiver.receive().await;

        match postcard::to_slice(&event, &mut buf) {
            Ok(data) => {
                if let Err(error) = write_ep.write(data).await {
                    warn!("Dropped {}, USB write failed: {}", event, error);
                }
            }
            Err(error) => error!("Failed to serialize postcard data: {}", error),
        }
    }
}

#[embassy_executor::task]
async fn usb_task(mut usb: embassy_usb::UsbDevice<'static, Driver<'static, USB>>) {
    usb.run().await;
}
