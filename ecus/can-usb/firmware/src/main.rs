#![no_std]
#![no_main]

mod board;
mod can;
mod leds;
mod usb;

use board::Board;
use can::CanBus;
use can_usb_protocol::{BusEvent, FirmwareVersion, Frame};
use defmt::{debug, info};
use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::{self, Channel};
use embassy_sync::priority_channel::{self, Min, PriorityChannel};
use embassy_time::Timer;
use usb::UsbDevice;
use {defmt_rtt as _, panic_probe as _};

/// The firmware version from `Cargo.toml`, for the device info.
const FIRMWARE_VERSION: FirmwareVersion = match FirmwareVersion::parse(env!("CARGO_PKG_VERSION")) {
    Some(version) => version,
    None => panic!("CARGO_PKG_VERSION is not major.minor.patch"),
};

// Define the synchronization channels between the CAN and USB tasks as 'static.
// Bus events keep the order they happened in.
const RX_CHANNEL_SIZE: usize = 4;
pub type RxChannel = Channel<ThreadModeRawMutex, BusEvent, RX_CHANNEL_SIZE>;
pub type RxSender = channel::Sender<'static, ThreadModeRawMutex, BusEvent, RX_CHANNEL_SIZE>;
pub type RxReceiver = channel::Receiver<'static, ThreadModeRawMutex, BusEvent, RX_CHANNEL_SIZE>;

const TX_CHANNEL_SIZE: usize = 4;
pub type TxChannel =
    priority_channel::PriorityChannel<ThreadModeRawMutex, Frame, Min, TX_CHANNEL_SIZE>;
pub type TxSender =
    priority_channel::Sender<'static, ThreadModeRawMutex, Frame, Min, TX_CHANNEL_SIZE>;
pub type TxReceiver =
    priority_channel::Receiver<'static, ThreadModeRawMutex, Frame, Min, TX_CHANNEL_SIZE>;

static RX_CHANNEL: RxChannel = Channel::new();
static TX_CHANNEL: TxChannel = PriorityChannel::new();

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let board = Board::new(embassy_stm32::init(board::config()));

    info!("can-usb firmware {} starting", FIRMWARE_VERSION);

    let can_bus = CanBus::new(
        board.can,
        board.transceiver,
        RX_CHANNEL.sender(),
        TX_CHANNEL.receiver(),
    );
    let usb_device = UsbDevice::new(board.usb, RX_CHANNEL.receiver(), TX_CHANNEL.sender());

    leds::start(spawner, board.led_rx, board.led_tx, board.led_err);
    usb_device.start(spawner);
    // Held for as long as the firmware runs: dropping it would put the
    // transceiver back in standby.
    let _transceiver = can_bus.start(spawner);

    loop {
        Timer::after_secs(10).await;
        debug!("Alive!");
    }
}
