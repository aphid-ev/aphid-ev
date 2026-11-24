//! What the interface does with its vendor control requests.
//!
//! The firmware's USB handler passes each request on the interface to a
//! [`Controller`], answers or STALLs as it says, and posts the
//! [`BusCommand`] it returns to the CAN task. The CAN task owns the
//! controller peripheral and carries the command out: it stops the bus,
//! clears what the command says, and starts it again if the command says to
//! run.
//!
//! | Request | Bus running | Bus stopped |
//! | --- | --- | --- |
//! | Set config | Restart with the new config, clear both queues | Store the config, clear both queues, stay stopped |
//! | Start bus | Restart with the current config, clear both queues | Start with the current config, clear both queues |
//! | Stop bus | Stop, discard queued frames to send | Nothing |
//! | USB reset | Restart with the default config unless already on it | Start with the default config |
//!
//! Clearing the queues on start and set config is what lets a program that
//! opens the interface long after it was plugged in start without the
//! events that queued up in between.

use crate::{Config, ControlRequest, DeviceInfo, PayloadError, UnknownRequest};
use core::fmt;

/// What the CAN task does with the bus. Every command stops the bus first
/// and discards the frames queued to send, so none of them goes out at a
/// bit rate or in a mode it was not sent for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct BusCommand {
    /// The config to start the bus with.
    pub config: Config,
    /// Whether to start the bus again. While it is stopped the transceiver
    /// is in standby and frames from the host are discarded.
    pub run: bool,
    /// Whether to clear the queue of events for the host, and the overrun
    /// count with it.
    pub clear_events: bool,
}

impl BusCommand {
    /// What the interface does at power-on: run with the default config.
    pub const POWER_ON: Self = Self {
        config: Config::DEFAULT,
        run: true,
        clear_events: false,
    };

    /// One command that does what `self` and then `next` would, for when a
    /// second request arrives before the CAN task took the first: the later
    /// config and run state, and the queues cleared if either cleared them.
    pub fn then(self, next: Self) -> Self {
        Self {
            config: next.config,
            run: next.run,
            clear_events: self.clear_events || next.clear_events,
        }
    }
}

/// Why the interface STALLs a control request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Rejection {
    /// The `bRequest` code is not a [`ControlRequest`].
    Unknown(UnknownRequest),
    /// The request came with the other data direction.
    WrongDirection(ControlRequest),
    /// `wValue` is not 0.
    Value(u16),
    /// A request without a payload came with data.
    UnexpectedData(ControlRequest),
    /// The payload does not decode, or the answer does not fit.
    Payload(PayloadError),
}

impl fmt::Display for Rejection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown(unknown) => unknown.fmt(f),
            Self::WrongDirection(request) => write!(f, "{request:?} with the wrong direction"),
            Self::Value(value) => write!(f, "wValue {value:#06x}, not 0"),
            Self::UnexpectedData(request) => write!(f, "{request:?} with data"),
            Self::Payload(error) => error.fmt(f),
        }
    }
}

impl core::error::Error for Rejection {}

impl From<PayloadError> for Rejection {
    fn from(error: PayloadError) -> Self {
        Self::Payload(error)
    }
}

/// The bus state the host has asked for: the config and whether the bus
/// runs. Starts at [`BusCommand::POWER_ON`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Controller {
    config: Config,
    running: bool,
}

impl Default for Controller {
    fn default() -> Self {
        Self::new()
    }
}

impl Controller {
    /// The state at power-on, which the CAN task starts in by itself.
    pub const fn new() -> Self {
        Self {
            config: BusCommand::POWER_ON.config,
            running: BusCommand::POWER_ON.run,
        }
    }

    /// The config the bus runs with, or will start with.
    pub fn config(&self) -> Config {
        self.config
    }

    /// Whether the bus runs.
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Handles a host-to-device request with `bRequest` `request`, `wValue`
    /// `value` and `data` from its data stage. `Ok(None)` accepts a request
    /// with nothing to do. On `Err` nothing changes, and the request is to be
    /// STALLed.
    pub fn control_out(
        &mut self,
        request: u8,
        value: u16,
        data: &[u8],
    ) -> Result<Option<BusCommand>, Rejection> {
        let request = Self::request(request, value, false)?;

        if request != ControlRequest::SetConfig && !data.is_empty() {
            return Err(Rejection::UnexpectedData(request));
        }

        match request {
            ControlRequest::SetConfig => {
                self.config = Config::decode(data)?;
                Ok(Some(self.command(true)))
            }
            ControlRequest::StartBus => {
                self.running = true;
                Ok(Some(self.command(true)))
            }
            ControlRequest::StopBus if self.running => {
                self.running = false;
                Ok(Some(self.command(false)))
            }
            ControlRequest::StopBus => Ok(None),
            ControlRequest::GetDeviceInfo => Err(Rejection::WrongDirection(request)),
        }
    }

    /// Handles a device-to-host request, writing the answer for the data
    /// stage to `buf` and returning its length. On `Err` the request is to be
    /// STALLed.
    pub fn control_in(
        &self,
        request: u8,
        value: u16,
        info: &DeviceInfo,
        buf: &mut [u8],
    ) -> Result<usize, Rejection> {
        match Self::request(request, value, true)? {
            ControlRequest::GetDeviceInfo => Ok(info.encode(buf)?),
            request => Err(Rejection::WrongDirection(request)),
        }
    }

    /// Handles a USB reset: the host that configured the bus is gone, so it
    /// goes back to the power-on default. Nothing to do when it is already
    /// there.
    pub fn reset(&mut self) -> Option<BusCommand> {
        let initial = Self::new();

        if *self == initial {
            return None;
        }

        *self = initial;
        Some(self.command(true))
    }

    fn request(request: u8, value: u16, device_to_host: bool) -> Result<ControlRequest, Rejection> {
        let request = ControlRequest::try_from(request).map_err(Rejection::Unknown)?;

        if request.is_device_to_host() != device_to_host {
            return Err(Rejection::WrongDirection(request));
        }
        if value != 0 {
            return Err(Rejection::Value(value));
        }

        Ok(request)
    }

    fn command(&self, clear_events: bool) -> BusCommand {
        BusCommand {
            config: self.config,
            run: self.running,
            clear_events,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Bitrate, FirmwareVersion, MAX_PACKET_SIZE, Mode, PROTOCOL_VERSION};

    const SET_CONFIG: u8 = ControlRequest::SetConfig.code();
    const START: u8 = ControlRequest::StartBus.code();
    const STOP: u8 = ControlRequest::StopBus.code();
    const INFO: u8 = ControlRequest::GetDeviceInfo.code();

    fn config(bitrate: Bitrate, mode: Mode) -> Config {
        Config { bitrate, mode }
    }

    fn payload(config: Config) -> Vec<u8> {
        let (buf, len) = config.encode();
        buf[..len].to_vec()
    }

    fn info() -> DeviceInfo {
        DeviceInfo {
            protocol_version: PROTOCOL_VERSION,
            firmware_version: FirmwareVersion {
                major: 0,
                minor: 1,
                patch: 0,
            },
            serial: [7; DeviceInfo::SERIAL_LEN],
        }
    }

    fn stopped() -> Controller {
        let mut controller = Controller::new();
        controller.control_out(STOP, 0, &[]).unwrap();
        controller
    }

    #[test]
    fn powers_on_running_with_the_default_config() {
        let controller = Controller::new();

        assert!(controller.is_running());
        assert_eq!(controller.config(), Config::default());
        assert_eq!(
            BusCommand::POWER_ON,
            BusCommand {
                config: Config::default(),
                run: true,
                clear_events: false,
            }
        );
    }

    #[test]
    fn set_config_while_running_restarts_with_it_and_clears() {
        let mut controller = Controller::new();
        let new = config(Bitrate::Kbit250, Mode::ListenOnly);

        let command = controller.control_out(SET_CONFIG, 0, &payload(new));

        assert_eq!(
            command,
            Ok(Some(BusCommand {
                config: new,
                run: true,
                clear_events: true,
            }))
        );
        assert_eq!(controller.config(), new);
        assert!(controller.is_running());
    }

    #[test]
    fn set_config_while_stopped_stores_it_and_stays_stopped() {
        let mut controller = stopped();
        let new = config(Bitrate::Mbit1, Mode::InternalLoopback);

        let command = controller.control_out(SET_CONFIG, 0, &payload(new));

        assert_eq!(
            command,
            Ok(Some(BusCommand {
                config: new,
                run: false,
                clear_events: true,
            }))
        );
        assert!(!controller.is_running());
    }

    #[test]
    fn start_uses_the_stored_config_and_clears() {
        let mut controller = stopped();
        let new = config(Bitrate::Kbit125, Mode::Normal);
        controller
            .control_out(SET_CONFIG, 0, &payload(new))
            .unwrap();

        assert_eq!(
            controller.control_out(START, 0, &[]),
            Ok(Some(BusCommand {
                config: new,
                run: true,
                clear_events: true,
            }))
        );
        assert!(controller.is_running());
    }

    /// A program that opens the interface sends start whatever the state,
    /// so that it begins with empty queues.
    #[test]
    fn start_while_running_restarts_and_clears() {
        let mut controller = Controller::new();

        assert_eq!(
            controller.control_out(START, 0, &[]),
            Ok(Some(BusCommand {
                config: Config::default(),
                run: true,
                clear_events: true,
            }))
        );
    }

    #[test]
    fn stop_keeps_the_events_and_the_config() {
        let mut controller = Controller::new();
        let new = config(Bitrate::Kbit250, Mode::Normal);
        controller
            .control_out(SET_CONFIG, 0, &payload(new))
            .unwrap();

        assert_eq!(
            controller.control_out(STOP, 0, &[]),
            Ok(Some(BusCommand {
                config: new,
                run: false,
                clear_events: false,
            }))
        );
        assert!(!controller.is_running());
        assert_eq!(controller.config(), new);
    }

    #[test]
    fn stop_while_stopped_does_nothing() {
        let mut controller = stopped();

        assert_eq!(controller.control_out(STOP, 0, &[]), Ok(None));
        assert_eq!(controller, stopped());
    }

    #[test]
    fn an_undecodable_config_is_rejected_and_changes_nothing() {
        let mut controller = Controller::new();
        let before = controller;

        for data in [&[][..], &[4, 0], &[0, 3], &[2], &[2, 0, 0], &[0xff, 0xff]] {
            assert!(
                matches!(
                    controller.control_out(SET_CONFIG, 0, data),
                    Err(Rejection::Payload(_))
                ),
                "{data:?}"
            );
            assert_eq!(controller, before);
        }
    }

    #[test]
    fn unknown_requests_are_rejected() {
        let mut controller = Controller::new();
        let mut buf = [0; MAX_PACKET_SIZE];

        for code in [0x00, 0x01, 0x0f, 0x14, 0xff] {
            assert_eq!(
                controller.control_out(code, 0, &[]),
                Err(Rejection::Unknown(UnknownRequest(code)))
            );
            assert_eq!(
                controller.control_in(code, 0, &info(), &mut buf),
                Err(Rejection::Unknown(UnknownRequest(code)))
            );
        }
        assert_eq!(controller, Controller::new());
    }

    #[test]
    fn requests_with_the_wrong_direction_are_rejected() {
        let mut controller = Controller::new();
        let mut buf = [0; MAX_PACKET_SIZE];

        assert_eq!(
            controller.control_out(INFO, 0, &[]),
            Err(Rejection::WrongDirection(ControlRequest::GetDeviceInfo))
        );
        for code in [SET_CONFIG, START, STOP] {
            assert!(matches!(
                controller.control_in(code, 0, &info(), &mut buf),
                Err(Rejection::WrongDirection(_))
            ));
        }
        assert_eq!(controller, Controller::new());
    }

    #[test]
    fn a_nonzero_value_is_rejected() {
        let mut controller = Controller::new();
        let mut buf = [0; MAX_PACKET_SIZE];

        assert_eq!(
            controller.control_out(START, 1, &[]),
            Err(Rejection::Value(1))
        );
        assert_eq!(
            controller.control_out(SET_CONFIG, 0x100, &payload(Config::default())),
            Err(Rejection::Value(0x100))
        );
        assert_eq!(
            controller.control_in(INFO, 2, &info(), &mut buf),
            Err(Rejection::Value(2))
        );
    }

    #[test]
    fn start_and_stop_with_data_are_rejected() {
        let mut controller = stopped();

        assert_eq!(
            controller.control_out(START, 0, &[0]),
            Err(Rejection::UnexpectedData(ControlRequest::StartBus))
        );
        assert!(!controller.is_running());
        assert_eq!(
            Controller::new().control_out(STOP, 0, &[0]),
            Err(Rejection::UnexpectedData(ControlRequest::StopBus))
        );
    }

    #[test]
    fn device_info_is_answered_with_its_encoding() {
        let mut buf = [0; MAX_PACKET_SIZE];

        let len = Controller::new()
            .control_in(INFO, 0, &info(), &mut buf)
            .unwrap();

        assert_eq!(DeviceInfo::decode(&buf[..len]), Ok(info()));
    }

    #[test]
    fn device_info_that_does_not_fit_is_rejected() {
        let mut buf = [0; 4];

        assert_eq!(
            Controller::new().control_in(INFO, 0, &info(), &mut buf),
            Err(Rejection::Payload(PayloadError::BufferTooSmall))
        );
    }

    #[test]
    fn usb_reset_returns_to_the_defaults() {
        let mut controller = stopped();
        controller
            .control_out(
                SET_CONFIG,
                0,
                &payload(config(Bitrate::Mbit1, Mode::ListenOnly)),
            )
            .unwrap();

        assert_eq!(
            controller.reset(),
            Some(BusCommand {
                config: Config::default(),
                run: true,
                clear_events: true,
            })
        );
        assert_eq!(controller, Controller::new());
    }

    /// Enumeration at power-on is a USB reset too, and must not restart a
    /// bus that already runs as it should.
    #[test]
    fn usb_reset_on_the_defaults_does_nothing() {
        let mut controller = Controller::new();

        assert_eq!(controller.reset(), None);
    }

    #[test]
    fn a_reset_after_a_config_change_restarts() {
        let mut controller = Controller::new();
        controller
            .control_out(
                SET_CONFIG,
                0,
                &payload(config(Bitrate::Kbit125, Mode::Normal)),
            )
            .unwrap();

        assert!(controller.reset().is_some());
    }

    #[test]
    fn a_later_command_wins_but_a_clear_is_kept() {
        let start = BusCommand {
            config: config(Bitrate::Kbit250, Mode::Normal),
            run: true,
            clear_events: true,
        };
        let stop = BusCommand {
            config: config(Bitrate::Kbit250, Mode::Normal),
            run: false,
            clear_events: false,
        };

        assert_eq!(
            start.then(stop),
            BusCommand {
                clear_events: true,
                ..stop
            }
        );
        assert_eq!(stop.then(start), start);
        assert_eq!(stop.then(stop), stop);
    }
}
