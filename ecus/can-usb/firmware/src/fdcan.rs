//! What FDCAN1 tells and takes that embassy-stm32 0.6 does not expose.
//!
//! The driver's `Properties::bus_error_mode` reads PSR, which clears the
//! last error code, and then drops that code. So the CAN task reads PSR and
//! ECR here, once per status, and keeps both the state and the error. The
//! driver's own `CanRx::read` also reads PSR when its FIFO is empty; each
//! read clears the code, so an error is seen by one of the two and never
//! twice.
//!
//! The driver can only leave bus-off automatically, at once. With that
//! turned off, [`recover`] does the same step by hand when the CAN task
//! decides to.

use can_usb_protocol::{BusError, BusState, ControllerStatus};
use embassy_stm32::pac::FDCAN1;
use embassy_stm32::pac::can::vals::Lec;

/// Reads FDCAN1's fault confinement state, error counters and last error
/// code. Call it only while FDCAN1 runs: its clock is off otherwise.
pub fn status() -> ControllerStatus {
    let psr = FDCAN1.psr().read();
    let ecr = FDCAN1.ecr().read();

    let state = if psr.bo() {
        BusState::BusOff
    } else if psr.ep() {
        BusState::ErrorPassive
    } else {
        BusState::ErrorActive
    };

    ControllerStatus {
        state,
        tec: ecr.tec(),
        rec: ecr.rec(),
        error: bus_error(psr.lec()),
    }
}

/// Starts the bus-off recovery: clears CCCR.INIT, which FDCAN1 set when it
/// went bus-off, keeping its configuration. FDCAN1 then waits for 128
/// sequences of 11 recessive bits before it takes part in the bus again.
pub fn recover() {
    FDCAN1.cccr().modify(|w| w.set_init(false));
}

/// The same mapping as embassy-stm32's for the errors `CanRx::read`
/// returns, so an error is named the same whichever of the two saw it. Both
/// name a bit error by the level FDCAN1 sent: Bit1Error is `BitRecessive`.
fn bus_error(lec: Lec) -> Option<BusError> {
    match lec {
        Lec::NO_ERROR | Lec::NO_CHANGE => None,
        Lec::STUFF => Some(BusError::Stuff),
        Lec::FORM => Some(BusError::Form),
        Lec::ACK => Some(BusError::Acknowledge),
        Lec::BIT_1 => Some(BusError::BitRecessive),
        Lec::BIT_0 => Some(BusError::BitDominant),
        Lec::CRC => Some(BusError::Crc),
    }
}
