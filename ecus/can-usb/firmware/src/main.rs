#![no_std]
#![no_main]

mod board;
mod can;
mod leds;
mod usb;

use board::Board;
use can::CanBus;
use can_usb_protocol::{BusCommand, BusEvent, FirmwareVersion, Frame};
use defmt::{debug, info};
use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::{self, Channel};
use embassy_sync::signal::Signal;
use embassy_time::Timer;
use usb::UsbDevice;
use {defmt_rtt as _, panic_probe as _};

/// The firmware version from `Cargo.toml`, for the device info.
const FIRMWARE_VERSION: FirmwareVersion = match FirmwareVersion::parse(env!("CARGO_PKG_VERSION")) {
    Some(version) => version,
    None => panic!("CARGO_PKG_VERSION is not major.minor.patch"),
};

// The queues between the CAN and USB tasks. Both are FIFOs, so bus events
// reach the host in the order they happened and frames from the host go out
// in the order it sent them.

/// Bus events from the CAN receive task to the USB write task.
///
/// A saturated 500 kbit/s bus carries about 4500 standard 8-byte frames a
/// second, and up to 10 000 empty ones. 128 events hold 28 ms of the first
/// and 13 ms of the second while the host is slow to read. The receive task never waits for room: what does not fit is
/// counted and reported as one `BusEvent::Overrun`.
///
/// RAM: 128 × 24 bytes (`size_of::<BusEvent>()`) plus the channel's
/// bookkeeping, 3104 bytes in the release build.
const EVENT_CHANNEL_SIZE: usize = 128;
pub type EventChannel = Channel<ThreadModeRawMutex, BusEvent, EVENT_CHANNEL_SIZE>;
pub type EventSender = channel::Sender<'static, ThreadModeRawMutex, BusEvent, EVENT_CHANNEL_SIZE>;
pub type EventReceiver =
    channel::Receiver<'static, ThreadModeRawMutex, BusEvent, EVENT_CHANNEL_SIZE>;

/// Frames from the USB read task to the CAN transmit task.
///
/// 64 frames are three full USB packets of the shortest frames, or twelve
/// of 8-byte ones. When the queue is full the USB read task waits, the
/// bulk OUT endpoint NAKs, and the host is held back without losing a
/// frame.
///
/// RAM: 64 × 20 bytes (`size_of::<Frame>()`) plus the channel's
/// bookkeeping, 1312 bytes in the release build.
const FRAME_CHANNEL_SIZE: usize = 64;
pub type FrameChannel = Channel<ThreadModeRawMutex, Frame, FRAME_CHANNEL_SIZE>;
pub type FrameSender = channel::Sender<'static, ThreadModeRawMutex, Frame, FRAME_CHANNEL_SIZE>;
pub type FrameReceiver = channel::Receiver<'static, ThreadModeRawMutex, Frame, FRAME_CHANNEL_SIZE>;

static EVENT_CHANNEL: EventChannel = Channel::new();
static FRAME_CHANNEL: FrameChannel = Channel::new();

/// The latest command from the USB control handler to the CAN task. The
/// handler merges a command into one the CAN task has not taken yet with
/// `BusCommand::then`, so none is lost.
pub type CommandSignal = Signal<ThreadModeRawMutex, BusCommand>;
static BUS_COMMANDS: CommandSignal = Signal::new();

/// Raised by the CAN task after it cleared a queue. The USB task on the
/// other end drops what it had already taken out of the queue and not yet
/// passed on.
pub type ClearSignal = Signal<ThreadModeRawMutex, ()>;
static EVENTS_CLEARED: ClearSignal = Signal::new();
static FRAMES_CLEARED: ClearSignal = Signal::new();

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let board = Board::new(embassy_stm32::init(board::config()));

    info!("can-usb firmware {} starting", FIRMWARE_VERSION);

    let can_bus = CanBus::new(
        board.can,
        board.transceiver,
        EVENT_CHANNEL.sender(),
        FRAME_CHANNEL.receiver(),
        &BUS_COMMANDS,
        &EVENTS_CLEARED,
        &FRAMES_CLEARED,
    );
    let usb_device = UsbDevice::new(
        board.usb,
        EVENT_CHANNEL.receiver(),
        FRAME_CHANNEL.sender(),
        &BUS_COMMANDS,
        &EVENTS_CLEARED,
        &FRAMES_CLEARED,
    );

    leds::start(spawner, board.led_rx, board.led_tx, board.led_err);
    usb_device.start(spawner);
    can_bus.start(spawner);

    loop {
        Timer::after_secs(10).await;
        debug!("Alive!");
    }
}
