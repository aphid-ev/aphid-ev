use crate::{ClearSignal, CommandSignal, EventReceiver, FIRMWARE_VERSION, FrameSender};
use can_usb_protocol::{
    BULK_IN_EP, BULK_IN_PACKET_LIMIT, BULK_OUT_EP, Batcher, BusCommand, Controller, DeviceInfo,
    DropLog, DropTracker, Frame, INTERFACE, MAX_PACKET_SIZE, MSOS_VENDOR_CODE, PRODUCT_ID,
    PROTOCOL_VERSION, VENDOR_ID, unpack,
};
use defmt::{debug, error, info, unwrap, warn};
use embassy_executor::Spawner;
use embassy_futures::select::{Either, select};
use embassy_stm32::usb::{Endpoint, In, Out};
use embassy_stm32::{peripherals::USB, usb::Driver};
use embassy_usb::control::{InResponse, OutResponse, Recipient, Request, RequestType};
use embassy_usb::driver::{Endpoint as _, EndpointAddress, EndpointIn, EndpointOut};
use embassy_usb::msos::{self, windows_version};
use embassy_usb::{Builder, Handler};
use static_cell::StaticCell;

static CONFIG_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
static BOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
static MSOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
static CONTROL_BUF: StaticCell<[u8; MAX_PACKET_SIZE]> = StaticCell::new();
static CONTROL_HANDLER: StaticCell<ControlHandler> = StaticCell::new();

/// Identifies this interface to Windows. Generated once and fixed from then
/// on: Windows caches the driver binding against this value, so changing it
/// orphans the binding on every machine that has already seen the device.
const DEVICE_INTERFACE_GUID: &str = "{19F2234B-041A-4811-B321-E288246821A7}";

pub struct UsbDevice {
    usb_driver: Driver<'static, USB>,
    event_receiver: EventReceiver,
    frame_sender: FrameSender,
    commands: &'static CommandSignal,
    events_cleared: &'static ClearSignal,
    frames_cleared: &'static ClearSignal,
}

impl UsbDevice {
    /// The control handler posts its commands to `commands`. The CAN task
    /// raises `events_cleared` and `frames_cleared` when it clears the
    /// queues, and the write and read tasks then drop what they hold.
    pub fn new(
        usb_driver: Driver<'static, USB>,
        event_receiver: EventReceiver,
        frame_sender: FrameSender,
        commands: &'static CommandSignal,
        events_cleared: &'static ClearSignal,
        frames_cleared: &'static ClearSignal,
    ) -> Self {
        Self {
            usb_driver,
            event_receiver,
            frame_sender,
            commands,
            events_cleared,
            frames_cleared,
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

        builder.handler(CONTROL_HANDLER.init(ControlHandler {
            controller: Controller::new(),
            info: DeviceInfo {
                protocol_version: PROTOCOL_VERSION,
                firmware_version: FIRMWARE_VERSION,
                // The same bytes as the serial number string above.
                serial: embassy_stm32::uid::uid(),
            },
            commands: self.commands,
        }));

        let usb = builder.build();

        // As in CanBus::start, spawn failure is a double-start programming
        // error the device can't recover from, so panic via defmt.
        spawner.spawn(unwrap!(read_task(
            read_ep,
            self.frame_sender,
            self.frames_cleared
        )));
        spawner.spawn(unwrap!(write_task(
            write_ep,
            self.event_receiver,
            self.events_cleared
        )));
        spawner.spawn(unwrap!(usb_task(usb)));
    }
}

/// Answers the vendor control requests on the interface, as the
/// [`Controller`] decides, and posts the bus commands they lead to for the
/// CAN task. It runs inside `UsbDevice::run`, so it never waits: a command
/// goes into a signal and the request is answered at once.
struct ControlHandler {
    controller: Controller,
    info: DeviceInfo,
    commands: &'static CommandSignal,
}

impl ControlHandler {
    /// Whether `req` is one of this interface's vendor requests. Others are
    /// left to the USB stack, which STALLs what nobody handles.
    fn is_ours(req: &Request) -> bool {
        req.request_type == RequestType::Vendor
            && req.recipient == Recipient::Interface
            && req.index == u16::from(INTERFACE)
    }

    fn post(&self, command: BusCommand) {
        let command = match self.commands.try_take() {
            Some(earlier) => earlier.then(command),
            None => command,
        };
        self.commands.signal(command);
    }
}

impl Handler for ControlHandler {
    /// A USB reset means the host that configured the bus is gone, so the
    /// bus goes back to the power-on default.
    fn reset(&mut self) {
        if let Some(command) = self.controller.reset() {
            info!("USB reset, bus back to the default config");
            self.post(command);
        }
    }

    fn control_out(&mut self, req: Request, data: &[u8]) -> Option<OutResponse> {
        if !Self::is_ours(&req) {
            return None;
        }

        Some(
            match self.controller.control_out(req.request, req.value, data) {
                Ok(Some(command)) => {
                    debug!("Control request {:#04x}: {}", req.request, command);
                    self.post(command);
                    OutResponse::Accepted
                }
                Ok(None) => OutResponse::Accepted,
                Err(rejection) => {
                    warn!("Rejected control request: {}", rejection);
                    OutResponse::Rejected
                }
            },
        )
    }

    fn control_in<'a>(&'a mut self, req: Request, buf: &'a mut [u8]) -> Option<InResponse<'a>> {
        if !Self::is_ours(&req) {
            return None;
        }

        Some(
            match self
                .controller
                .control_in(req.request, req.value, &self.info, buf)
            {
                Ok(len) => InResponse::Accepted(&buf[..len]),
                Err(rejection) => {
                    warn!("Rejected control request: {}", rejection);
                    InResponse::Rejected
                }
            },
        )
    }
}

#[embassy_executor::task]
async fn read_task(
    mut read_ep: Endpoint<'static, USB, Out>,
    frame_sender: FrameSender,
    frames_cleared: &'static ClearSignal,
) {
    let mut buf = [0u8; MAX_PACKET_SIZE];

    loop {
        read_ep.wait_enabled().await;
        info!("USB connected");

        while let Ok(len) = read_ep.read(&mut buf).await {
            // A clear before this packet arrived does not concern it.
            frames_cleared.reset();

            // A packet carries a whole number of frames back to back. A tail
            // that does not decode ends the packet: the frames before it are
            // sent, the rest is dropped.
            //
            // A full frame queue makes this wait, and the endpoint is not
            // read again until it has room: the host's next packet is NAKed
            // and no frame is lost. When the CAN task clears the queue
            // meanwhile, the rest of the packet goes with it.
            let mut frames = unpack::<Frame>(&buf[..len]);
            while let Some(frame) = frames.next() {
                match frame {
                    Ok(frame) => {
                        // The clear goes first: the CAN task empties the
                        // queue before it raises the signal, so when both
                        // are ready, send would succeed and queue a frame
                        // from before the clear.
                        let cleared = select(frames_cleared.wait(), frame_sender.send(frame)).await;
                        if let Either::First(()) = cleared {
                            let rest = frames.filter(Result::is_ok).count();
                            info!("Discarded {} frames from USB", rest + 1);
                            break;
                        }
                    }
                    Err(error) => error!("Dropped the rest of a USB packet: {}", error),
                }
            }
        }

        info!("USB disconnected");
    }
}

#[embassy_executor::task]
async fn write_task(
    mut write_ep: Endpoint<'static, USB, In>,
    event_receiver: EventReceiver,
    events_cleared: &'static ClearSignal,
) {
    // Every packet stays one byte short of the endpoint's maximum, so each
    // ends its USB transfer and no zero-length packet is needed. See
    // BULK_IN_PACKET_LIMIT. The event that does not fit one packet stays in
    // the batcher and opens the next, so events keep their order.
    let mut batcher = Batcher::new(BULK_IN_PACKET_LIMIT);
    let mut drops = DropTracker::new();

    loop {
        // A clear raised since the last pass makes the carried event one
        // from before it.
        if events_cleared.try_take().is_some() {
            debug!("Discarded {} events on a clear", batcher.discard());
        }

        // Wait for one event unless one is carried over, then take whatever
        // else is queued without waiting. No timer: on a quiet bus each
        // event goes out at once.
        let first = if batcher.has_carry() {
            None
        } else {
            Some(event_receiver.receive().await)
        };
        // A clear while waiting for `first` found the batcher empty and
        // came before `first` was queued, so it concerns nothing held here.
        events_cleared.reset();
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
        //
        // When the CAN task clears the event queue while this waits, the
        // packet and the carried event are from before the clear and go
        // too. write() copies the packet only in the poll that returns, so
        // cancelling it sends nothing. The clear is polled first, so a write
        // that could complete in the same wake is not tried. A packet the
        // endpoint already holds is beyond reach and still goes to the host.
        let written = select(events_cleared.wait(), write_ep.write(packet)).await;
        let log = match written {
            Either::First(()) => {
                debug!("Discarded {} events on a clear", batcher.discard());
                None
            }
            Either::Second(Ok(())) => drops.written(),
            Either::Second(Err(_)) => drops.failed(batcher.count()),
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
