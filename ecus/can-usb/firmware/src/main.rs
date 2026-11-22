#![no_std]
#![no_main]

mod can;
mod usb;

use can::CanBus;
use can_usb_protocol::{BusEvent, Frame};
use defmt::{debug, info};
use embassy_executor::Spawner;
use embassy_stm32::can::{CanConfigurator, IT0InterruptHandler, IT1InterruptHandler};
use embassy_stm32::time::Hertz;
use embassy_stm32::usb::{Driver, InterruptHandler};
use embassy_stm32::{bind_interrupts, peripherals::*, Config};
use embassy_sync::priority_channel::PriorityChannel;
use embassy_sync::{
    blocking_mutex::raw::ThreadModeRawMutex,
    priority_channel::{self, Min},
};
use embassy_time::Timer;
use usb::UsbDevice;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct UsbIrqs {
    USB_LP =>  InterruptHandler<USB>;
});

bind_interrupts!(struct CanIrqs {
    FDCAN1_IT0 => IT0InterruptHandler<FDCAN1>;
    FDCAN1_IT1 => IT1InterruptHandler<FDCAN1>;
});

// Define the synchronization channels between the CAN and USB tasks as 'static
const RX_CHANNEL_SIZE: usize = 4;
pub type RxChannel =
    priority_channel::PriorityChannel<ThreadModeRawMutex, BusEvent, Min, RX_CHANNEL_SIZE>;
pub type RxSender =
    priority_channel::Sender<'static, ThreadModeRawMutex, BusEvent, Min, RX_CHANNEL_SIZE>;
pub type RxReceiver =
    priority_channel::Receiver<'static, ThreadModeRawMutex, BusEvent, Min, RX_CHANNEL_SIZE>;

const TX_CHANNEL_SIZE: usize = 4;
pub type TxChannel =
    priority_channel::PriorityChannel<ThreadModeRawMutex, Frame, Min, TX_CHANNEL_SIZE>;
pub type TxSender =
    priority_channel::Sender<'static, ThreadModeRawMutex, Frame, Min, TX_CHANNEL_SIZE>;
pub type TxReceiver =
    priority_channel::Receiver<'static, ThreadModeRawMutex, Frame, Min, TX_CHANNEL_SIZE>;

static RX_CHANNEL: RxChannel = PriorityChannel::new();
static TX_CHANNEL: TxChannel = PriorityChannel::new();

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let mut config = Config::default();
    {
        use embassy_stm32::rcc::*;
        config.rcc.hse = Some(Hse {
            freq: Hertz(16_000_000),
            mode: HseMode::Oscillator,
        });
        config.rcc.pll = Some(Pll {
            source: PllSource::HSE,
            prediv: PllPreDiv::DIV4,
            mul: PllMul::MUL72,
            divp: None,
            divr: Some(PllRDiv::DIV2), // PLLR -> 144 MHz
            divq: Some(PllQDiv::DIV6), // PLLQ -> 48 MHz
        });
        config.rcc.sys = Sysclk::PLL1_R;
        // Range 1 boost mode. Not strictly needed at 144 MHz — that is within
        // Range 1 normal mode, and embassy picks the same four flash wait
        // states either way — but it leaves headroom towards the 170 MHz
        // ceiling. Keep in sync with can_bootloader, which sets PWR_CR5
        // by hand.
        config.rcc.boost = true;
        config.rcc.mux.clk48sel = mux::Clk48sel::PLL1_Q;
        config.rcc.mux.fdcansel = mux::Fdcansel::PLL1_Q;
    }
    let p = embassy_stm32::init(config);

    info!("Starting...");

    let usb_driver = Driver::new(p.USB, UsbIrqs, p.PA12, p.PA11);
    let can_driver = CanConfigurator::new(p.FDCAN1, p.PB8, p.PB9, CanIrqs);

    let can_bus = CanBus::new(can_driver, RX_CHANNEL.sender(), TX_CHANNEL.receiver());
    let usb_device = UsbDevice::new(usb_driver, RX_CHANNEL.receiver(), TX_CHANNEL.sender());

    usb_device.start(spawner);
    can_bus.start(spawner);

    loop {
        Timer::after_secs(10).await;
        debug!("Alive!");
    }
}
