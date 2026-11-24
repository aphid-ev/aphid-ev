use crate::{EventReceiver, FrameSender};
use can_usb_protocol::{
    BULK_IN_EP, BULK_IN_PACKET_LIMIT, BULK_OUT_EP, Batcher, DropLog, DropTracker, Frame, INTERFACE,
    MAX_PACKET_SIZE, MSOS_VENDOR_CODE, PRODUCT_ID, VENDOR_ID, unpack,
};
use defmt::{error, info, unwrap};
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
    event_receiver: EventReceiver,
    frame_sender: FrameSender,
}

impl UsbDevice {
    pub fn new(
        usb_driver: Driver<'static, USB>,
        event_receiver: EventReceiver,
        frame_sender: FrameSender,
    ) -> Self {
        Self {
            usb_driver,
            event_receiver,
            frame_sender,
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
        spawner.spawn(unwrap!(read_task(read_ep, self.frame_sender)));
        spawner.spawn(unwrap!(write_task(write_ep, self.event_receiver)));
        spawner.spawn(unwrap!(usb_task(usb)));
    }
}

#[embassy_executor::task]
async fn read_task(mut read_ep: Endpoint<'static, USB, Out>, frame_sender: FrameSender) {
    let mut buf = [0u8; MAX_PACKET_SIZE];

    loop {
        read_ep.wait_enabled().await;
        info!("USB connected");

        while let Ok(len) = read_ep.read(&mut buf).await {
            // A packet carries a whole number of frames back to back. A tail
            // that does not decode ends the packet: the frames before it are
            // sent, the rest is dropped.
            //
            // A full frame queue makes this wait, and the endpoint is not
            // read again until it has room: the host's next packet is NAKed
            // and no frame is lost.
            for frame in unpack::<Frame>(&buf[..len]) {
                match frame {
                    Ok(frame) => frame_sender.send(frame).await,
                    Err(error) => error!("Dropped the rest of a USB packet: {}", error),
                }
            }
        }

        info!("USB disconnected");
    }
}

#[embassy_executor::task]
async fn write_task(mut write_ep: Endpoint<'static, USB, In>, event_receiver: EventReceiver) {
    // Every packet stays one byte short of the endpoint's maximum, so each
    // ends its USB transfer and no zero-length packet is needed. See
    // BULK_IN_PACKET_LIMIT. The event that does not fit one packet stays in
    // the batcher and opens the next, so events keep their order.
    let mut batcher = Batcher::new(BULK_IN_PACKET_LIMIT);
    let mut drops = DropTracker::new();

    loop {
        // Wait for one event unless one is carried over, then take whatever
        // else is queued without waiting. No timer: on a quiet bus each
        // event goes out at once.
        let first = if batcher.has_carry() {
            None
        } else {
            Some(event_receiver.receive().await)
        };
        let queued = core::iter::from_fn(|| event_receiver.try_receive().ok());
        let packet = batcher.build(first.into_iter().chain(queued), |event, error| {
            error!("Failed to encode {}: {}", event, error)
        });

        if packet.is_empty() {
            continue;
        }

        // The endpoint has one packet buffer. write waits until the host
        // has collected the previous packet, copies this one in and returns,
        // so this task builds the next packet while the host has yet to
        // collect this one, and then waits for it. Events that arrive
        // meanwhile go into the packet after that. A packet that write has
        // accepted but the host never collects, because of a USB reset or
        // an unplug, is lost and not counted.
        //
        // While no host has the device configured, the write fails at once
        // and the packet's events are discarded. That keeps the event queue
        // draining, so a host that connects late gets current events rather
        // than an overrun for frames from before it was there. Logged once
        // when it starts, and once with the count when writes go through
        // again.
        let log = match write_ep.write(packet).await {
            Ok(()) => drops.written(),
            Err(_) => drops.failed(batcher.count()),
        };

        match log {
            Some(DropLog::Started) => info!("USB write failed, dropping events"),
            Some(DropLog::Resumed { dropped }) => {
                info!("USB writes resumed, {} events dropped", dropped)
            }
            None => {}
        }
    }
}

#[embassy_executor::task]
async fn usb_task(mut usb: embassy_usb::UsbDevice<'static, Driver<'static, USB>>) {
    usb.run().await;
}
