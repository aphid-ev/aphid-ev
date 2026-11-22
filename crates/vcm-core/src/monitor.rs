//! Checks the monitor MCU runs on the main MCU's behaviour. Each check takes
//! one tick per control cycle, so tests drive time by calling `tick`.

use crate::throttle::{Throttle, ThrottleError};
use crate::timeout::{Timeout, TimeoutError};
use ev_can::Message;

#[derive(Debug, PartialEq)]
pub enum MonitorError {
    Throttle(ThrottleError),
    Timeout,
}

impl From<TimeoutError> for MonitorError {
    fn from(_value: TimeoutError) -> Self {
        MonitorError::Timeout
    }
}

/// Reads the accelerator pedal independently of the main MCU. A bad reading
/// counts as zero throttle until it has persisted for `timeout` ticks, then
/// it is an error.
pub struct ThrottleMonitor<'a> {
    throttle: &'a Throttle,
    timeout: Timeout,
}

impl<'a> ThrottleMonitor<'a> {
    pub fn new(throttle: &'a Throttle, timeout: usize) -> Self {
        Self {
            throttle,
            timeout: Timeout::new(timeout),
        }
    }

    /// The throttle position for this tick, from the two sensor voltages in
    /// millivolts.
    pub fn check(&mut self, sensor1: u16, sensor2: u16) -> Result<u16, MonitorError> {
        match self.throttle.position(sensor1, sensor2) {
            Err(err) => match self.timeout.tick() {
                Err(TimeoutError::Elapsed) => Err(MonitorError::Throttle(err)),
                Ok(_) => Ok(0),
            },
            Ok(position) => {
                self.timeout.reset();
                Ok(position)
            }
        }
    }
}

/// Watches the main MCU's torque requests on EV-CAN. It is an error when no
/// valid request has arrived for `timeout` ticks.
///
/// Incomplete: it does not yet compare the requested torque with the
/// throttle position.
pub struct TorqueMonitor {
    frame_timeout: Timeout,
}

impl TorqueMonitor {
    pub fn new(frame_timeout: usize) -> Self {
        Self {
            frame_timeout: Timeout::new(frame_timeout),
        }
    }

    pub fn tick(&mut self) -> Result<(), MonitorError> {
        self.frame_timeout.tick()?;
        Ok(())
    }

    /// Feed every decoded EV-CAN message. Torque requests restart the
    /// timeout; other messages are ignored.
    pub fn message(&mut self, message: &Message) {
        if let Message::TorqueRequest { .. } = message {
            self.frame_timeout.reset();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn throttle_errors_are_tolerated_until_the_timeout() {
        let throttle = Throttle::new((1000, 2000), (3000, 4000), 1500);
        let mut monitor = ThrottleMonitor::new(&throttle, 2);

        assert_eq!(monitor.check(500, 3500), Ok(0));
        assert_eq!(monitor.check(500, 3500), Ok(0));
        assert_eq!(
            monitor.check(500, 3500),
            Err(MonitorError::Throttle(ThrottleError::SensorOutOfRange))
        );

        assert_eq!(monitor.check(1500, 3500), Ok(32767));
    }

    #[test]
    fn a_good_reading_resets_the_throttle_timeout() {
        let throttle = Throttle::new((1000, 2000), (3000, 4000), 1500);
        let mut monitor = ThrottleMonitor::new(&throttle, 1);

        assert_eq!(monitor.check(500, 3500), Ok(0));
        assert_eq!(monitor.check(1500, 3500), Ok(32767));
        assert_eq!(monitor.check(500, 3500), Ok(0));
    }

    /// The prototype never reset the timeout, so it reported an error ten
    /// ticks after start however often requests arrived.
    #[test]
    fn torque_requests_keep_the_monitor_alive() {
        let mut monitor = TorqueMonitor::new(2);
        let request = Message::TorqueRequest {
            torque: 0,
            counter: 0,
        };

        for _ in 0..10 {
            assert_eq!(monitor.tick(), Ok(()));
            monitor.message(&request);
        }
    }

    #[test]
    fn missing_torque_requests_time_out() {
        let mut monitor = TorqueMonitor::new(2);

        assert_eq!(monitor.tick(), Ok(()));
        assert_eq!(monitor.tick(), Ok(()));
        monitor.message(&Message::VcmKeepalive2);
        assert_eq!(monitor.tick(), Err(MonitorError::Timeout));
    }
}
