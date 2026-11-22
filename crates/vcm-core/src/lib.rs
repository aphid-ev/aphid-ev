//! Vehicle control logic for the VCM, shared by its main and monitor MCUs
//! and tested on the host.
//!
//! Safety-relevant: throttle plausibility, the direction rules and the
//! monitor checks live here. Change them only with a test that covers the
//! change.

// Tests link the standard test harness, so std is only excluded otherwise.
#![cfg_attr(not(test), no_std)]

pub mod direction;
pub mod input;
pub mod link;
pub mod monitor;
pub mod throttle;
pub mod timeout;
