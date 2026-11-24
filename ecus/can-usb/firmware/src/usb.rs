use crate::{RxReceiver, TxSender};
use can_usb_protocol::{
    BULK_IN_EP, BULK_OUT_EP, Frame, INTERFACE, MAX_PACKET_SIZE, MSOS_VENDOR_CODE, PRODUCT_ID,
    Packer, VENDOR_ID, unpack,
};
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
static CONTROL_BUF: StaticCell<[u8; MAX_PACKET_SIZE]> = StaticCell::new();

/// Identifies this interface to Windows. Generated once and fixed from then
/// on: Windows caches the driver binding against this value, so changing it
/// orphans the binding on every machine that has already seen the device.
const DEVICE_INTERFACE_GUID: &str = "{19F2234B-041A-4811-B321-E288246821A7}";

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
        let mut config = embassy_usb::Config::new(VENDOR_ID, PRODUCT_ID);
        config.manufacturer = Some("Aphid-ev");
        config.product = Some("can-interface");
        // The 96-bit unique ID as 24 uppercase hex digits, byte 0 (lowest
        // address) first: the same bytes, in the same order, as
        // DeviceInfo::serial.
        config.serial_number = Some(embassy_stm32::uid::uid_hex());
        config.max_power = 100;
        config.max_packet_size_0 = MAX_PACKET_SIZE as u8;

        let config_descriptor = CONFIG_DESCRIPTOR.init([0; 256]);
        let bos_descriptor = BOS_DESCRIPTOR.init([0; 256]);
        let msos_descriptor = MSOS_DESCRIPTOR.init([0; 256]);
        let control_buf = CONTROL_BUF.init([0; MAX_PACKET_SIZE]);

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
        // opens them by address.
        let mut function = builder.function(0xFF, 0, 0);
        function.msos_feature(msos::CompatibleIdFeatureDescriptor::new("WINUSB", ""));
        function.msos_feature(msos::RegistryPropertyFeatureDescriptor::new(
            "DeviceInterfaceGUIDs",
            msos::PropertyData::RegMultiSz(&[DEVICE_INTERFACE_GUID]),
        ));
        let mut interface = function.interface();
        // The host claims the interface by this number. embassy numbers
        // interfaces in the order they are created, so this one is 0 only
        // while it is the first.
        defmt::assert_eq!(interface.interface_number().0, INTERFACE);
        let mut alt = interface.alt_setting(0xFF, 0, 0, None);
        let read_ep = alt.endpoint_bulk_out(
            Some(EndpointAddress::from(BULK_OUT_EP)),
            MAX_PACKET_SIZE as u16,
        );
        let write_ep = alt.endpoint_bulk_in(
            Some(EndpointAddress::from(BULK_IN_EP)),
            MAX_PACKET_SIZE as u16,
        );
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
    let mut buf = [0u8; MAX_PACKET_SIZE];

    loop {
        read_ep.wait_enabled().await;
        info!("USB connected");

        while let Ok(len) = read_ep.read(&mut buf).await {
            // A packet carries a whole number of frames back to back. A tail
            // that does not decode ends the packet: the frames before it are
            // sent, the rest is dropped.
            for frame in unpack::<Frame>(&buf[..len]) {
                match frame {
                    Ok(frame) => tx_sender.send(frame).await,
                    Err(error) => error!("Dropped the rest of a USB packet: {}", error),
                }
            }
        }

        info!("USB disconnected");
    }
}

#[embassy_executor::task]
async fn write_task(mut write_ep: Endpoint<'static, USB, In>, rx_receiver: RxReceiver) {
    let mut packer = Packer::new();

    loop {
        let event = rx_receiver.receive().await;

        // One event per packet for now. Every event fits an empty packet,
        // which the protocol crate's tests guarantee.
        packer.clear();

        match packer.push(&event) {
            Ok(()) => {
                if let Err(error) = write_ep.write(packer.as_bytes()).await {
                    warn!("Dropped {}, USB write failed: {}", event, error);
                }
            }
            Err(error) => error!("Failed to encode {}: {}", event, error),
        }
    }
}

#[embassy_executor::task]
async fn usb_task(mut usb: embassy_usb::UsbDevice<'static, Driver<'static, USB>>) {
    usb.run().await;
}
