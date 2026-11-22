use core::num::{NonZeroU16, NonZeroU8};

use crate::pac::{FDCAN, FDCAN1, GPIOB, RCC};
use boot_protocol::{Error, Frame, Result};
use fdcan::{
    config::NominalBitTiming,
    filter::{StandardFilter, StandardFilterSlot},
    frame::{FrameFormat, RxFrameInfo, TxFrameHeader},
    id::{Id, StandardId},
    NormalOperationMode, ReceiveOverrun,
};
use nb::block;

struct CanInner {
    #[allow(dead_code)]
    fdcan: FDCAN,
}

pub struct Can {
    fdcan: fdcan::FdCan<CanInner, NormalOperationMode>,
}

// FDCAN clocked by PLLQ (48 MHz)
// prescaler = 6
// seg1 = 13
// seg2 = 2
// sjw = 1
//  t_q = prescaler / PLLQ  ->  12.5 us
//  t_sync = t_q * 1        ->  12.5 us
//  t_seg1 = t_q * seg1     -> 162.5 us
//  t_seg2 = t_q * seg2     ->  25.0 us
//  t_bit  = t_sync + t_seg1 + t_seg2 -> 200 us
//  baud_rate = 1 / t_bit   -> 500 000 kbit/s
//  sample = (t_sync + t_seg1) / t_bit -> 87.5%
const BTR: NominalBitTiming = NominalBitTiming {
    prescaler: NonZeroU16::new(6).unwrap(),
    seg1: NonZeroU8::new(13).unwrap(),
    seg2: NonZeroU8::new(2).unwrap(),
    sync_jump_width: NonZeroU8::new(1).unwrap(),
};

impl Can {
    pub fn new(fdcan: FDCAN, rcc: &mut RCC, gpiob: &mut GPIOB) -> Self {
        // Enable GPIOB and FDCAN clocks
        rcc.ahb2enr().modify(|_, w| w.gpioben().enabled());
        rcc.apb1enr1().modify(|_, w| w.fdcanen().enabled());

        // Set GPIOB PB8 & PB9 to alternate mode 9, high speed
        gpiob.moder().modify(|_, w| w.moder8().alternate());
        gpiob.moder().modify(|_, w| w.moder9().alternate());
        gpiob.afrh().modify(|_, w| w.afrh8().af9());
        gpiob.afrh().modify(|_, w| w.afrh9().af9());
        gpiob.ospeedr().modify(|_, w| w.ospeedr8().high_speed());
        gpiob.ospeedr().modify(|_, w| w.ospeedr9().high_speed());

        // Set FDCAN to use PLLQ (48 MHz)
        rcc.ccipr().modify(|_, w| w.fdcansel().pllq());

        let mut fdcan = fdcan::FdCan::new(CanInner { fdcan }).into_config_mode();
        fdcan.set_protocol_exception_handling(false);
        fdcan.set_nominal_bit_timing(BTR);
        fdcan.set_standard_filter(
            StandardFilterSlot::_0,
            StandardFilter::accept_all_into_fifo0(),
        ); // TODO: Create a filter for bootloader frames

        let fdcan = fdcan.into_normal();

        Self { fdcan }
    }

    pub fn reset(&mut self, rcc: &mut RCC, gpiob: &mut GPIOB) {
        // Reset GPIOB PB8 & PB9 to default
        gpiob.moder().reset();
        gpiob.afrh().reset();
        gpiob.ospeedr().reset();

        // Disable GPIOB and FDCAN clocks
        rcc.ahb2enr().modify(|_, w| w.gpioben().disabled());
        rcc.apb1enr1().modify(|_, w| w.fdcanen().disabled());

        // TODO: Reset FDCAN
    }

    /// Returns a frame if one is waiting, and `None` otherwise.
    ///
    /// Deliberately not wrapped in `nb::block!`: every caller polls this in a
    /// loop that also has to make progress on something else. Blocking here
    /// starves the boot timeout in `state_wait`, which then never expires on
    /// a quiet bus and leaves the application unbooted.
    pub fn receive(&mut self) -> Option<Frame> {
        let mut buffer = [0u8; 8];

        match self.fdcan.receive0(&mut buffer) {
            Ok(ReceiveOverrun::NoOverrun(header)) => from_fdcan(header, &buffer),
            _ => None,
        }
    }

    pub fn send(&mut self, frame: &Frame) -> Result<()> {
        let header = header(frame).ok_or(Error::Header)?;

        block!(self.fdcan.transmit(header, frame.data())).map_err(|_| Error::FdCan)?;

        Ok(())
    }
}

/// Classic frames with a standard identifier are the only kind the protocol
/// uses; anything else is not for the bootloader.
fn from_fdcan(header: RxFrameInfo, buffer: &[u8; 8]) -> Option<Frame> {
    match (header.frame_format, header.id) {
        (FrameFormat::Standard, Id::Standard(id)) => {
            Frame::new(id.as_raw(), buffer.get(..header.len as usize)?)
        }
        _ => None,
    }
}

fn header(frame: &Frame) -> Option<TxFrameHeader> {
    Some(TxFrameHeader {
        len: frame.data().len() as u8,
        frame_format: FrameFormat::Standard,
        id: Id::Standard(StandardId::new(frame.id())?),
        bit_rate_switching: false,
        marker: None,
    })
}

unsafe impl fdcan::Instance for CanInner {
    const REGISTERS: *mut fdcan::RegisterBlock = FDCAN1::ptr() as *mut _;
}

unsafe impl fdcan::message_ram::Instance for CanInner {
    const MSG_RAM: *mut fdcan::message_ram::RegisterBlock = (0x4000_a400 as *mut _);
}
