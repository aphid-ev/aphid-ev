//! Payloads of the vendor control requests (see
//! [`ControlRequest`](crate::ControlRequest)).

use core::fmt;
use serde::{Deserialize, Serialize};

/// What the interface reports about itself in answer to
/// [`ControlRequest::GetDeviceInfo`](crate::ControlRequest::GetDeviceInfo).
///
/// The host reads this at open and compares `protocol_version` with
/// [`PROTOCOL_VERSION`](crate::PROTOCOL_VERSION) before it trusts anything
/// else the device sends. That only works while `protocol_version` stays the
/// first field: never move it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct DeviceInfo {
    pub protocol_version: u16,
    pub firmware_version: FirmwareVersion,
    /// The microcontroller's 96-bit unique ID, unique per device.
    pub serial: [u8; DeviceInfo::SERIAL_LEN],
}

impl DeviceInfo {
    pub const SERIAL_LEN: usize = 12;
}

/// Semantic version of the firmware, from its `Cargo.toml`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct FirmwareVersion {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

impl FirmwareVersion {
    /// Parses `major.minor.patch`, ignoring a pre-release or build suffix
    /// (`-rc.1`, `+abc`). `const`, so a firmware can parse
    /// `env!("CARGO_PKG_VERSION")` at compile time:
    ///
    /// ```
    /// # use can_usb_protocol::FirmwareVersion;
    /// const VERSION: FirmwareVersion = match FirmwareVersion::parse("1.2.3") {
    ///     Some(version) => version,
    ///     None => panic!("not a version"),
    /// };
    /// assert_eq!(VERSION.minor, 2);
    /// ```
    pub const fn parse(version: &str) -> Option<Self> {
        let bytes = version.as_bytes();
        let mut parts = [0u16; 3];
        let mut part = 0;
        let mut digits = 0;
        let mut i = 0;

        while i < bytes.len() {
            match bytes[i] {
                digit @ b'0'..=b'9' => {
                    let value = parts[part] as u32 * 10 + (digit - b'0') as u32;

                    if value > u16::MAX as u32 {
                        return None;
                    }

                    parts[part] = value as u16;
                    digits += 1;
                }
                b'.' if digits > 0 && part < 2 => {
                    part += 1;
                    digits = 0;
                }
                b'-' | b'+' => break,
                _ => return None,
            }

            i += 1;
        }

        if part != 2 || digits == 0 {
            return None;
        }

        Some(Self {
            major: parts[0],
            minor: parts[1],
            patch: parts[2],
        })
    }
}

impl fmt::Display for FirmwareVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Bus settings, sent with
/// [`ControlRequest::SetConfig`](crate::ControlRequest::SetConfig). The
/// interface starts with [`Config::default`], so a host that never configures
/// it gets 500 kbit/s in normal mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Config {
    pub bitrate: Bitrate,
    pub mode: Mode,
}

/// Classic CAN bit rates the interface supports. Encoded by position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Bitrate {
    Kbit125,
    Kbit250,
    #[default]
    Kbit500,
    Mbit1,
}

impl Bitrate {
    pub const ALL: [Self; 4] = [Self::Kbit125, Self::Kbit250, Self::Kbit500, Self::Mbit1];

    pub const fn bits_per_second(self) -> u32 {
        match self {
            Self::Kbit125 => 125_000,
            Self::Kbit250 => 250_000,
            Self::Kbit500 => 500_000,
            Self::Mbit1 => 1_000_000,
        }
    }

    /// The supported rate of exactly `bits_per_second`, if there is one.
    pub fn from_bits_per_second(bits_per_second: u32) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|rate| rate.bits_per_second() == bits_per_second)
    }
}

impl fmt::Display for Bitrate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} kbit/s", self.bits_per_second() / 1000)
    }
}

/// How the interface takes part in the bus. Encoded by position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Mode {
    /// Receives, transmits and acknowledges like any other node.
    #[default]
    Normal,
    /// Receives only: never transmits and never acknowledges, so it can be
    /// connected to a running vehicle without disturbing it.
    ListenOnly,
    /// Frames sent are received back internally and never reach the bus.
    InternalLoopback,
}

impl Mode {
    pub const ALL: [Self; 3] = [Self::Normal, Self::ListenOnly, Self::InternalLoopback];
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MAX_PACKET_SIZE;

    fn device_info() -> DeviceInfo {
        DeviceInfo {
            protocol_version: 1,
            firmware_version: FirmwareVersion {
                major: 0,
                minor: 2,
                patch: 300,
            },
            serial: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
        }
    }

    #[test]
    fn device_info_round_trips_through_postcard() {
        let bytes = postcard::to_allocvec(&device_info()).unwrap();

        assert_eq!(
            postcard::from_bytes::<DeviceInfo>(&bytes).unwrap(),
            device_info()
        );
    }

    /// The host reads this with a separately built firmware on the other
    /// end, so the bytes are a contract.
    #[test]
    fn device_info_wire_encoding_is_stable() {
        assert_eq!(
            postcard::to_allocvec(&device_info()).unwrap(),
            vec![
                1, // protocol version, varint
                0, 2, 0xac, 0x02, // firmware version 0.2.300, varints
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, // serial, raw bytes
            ]
        );
    }

    /// The control endpoint's buffer is one packet long.
    #[test]
    fn largest_device_info_fits_a_control_transfer() {
        let largest = DeviceInfo {
            protocol_version: u16::MAX,
            firmware_version: FirmwareVersion {
                major: u16::MAX,
                minor: u16::MAX,
                patch: u16::MAX,
            },
            serial: [0xff; DeviceInfo::SERIAL_LEN],
        };

        assert!(postcard::to_allocvec(&largest).unwrap().len() <= MAX_PACKET_SIZE);
    }

    #[test]
    fn config_wire_encoding_is_stable() {
        for (bitrate_tag, bitrate) in Bitrate::ALL.into_iter().enumerate() {
            for (mode_tag, mode) in Mode::ALL.into_iter().enumerate() {
                assert_eq!(
                    postcard::to_allocvec(&Config { bitrate, mode }).unwrap(),
                    vec![bitrate_tag as u8, mode_tag as u8]
                );
            }
        }
    }

    #[test]
    fn config_round_trips_through_postcard() {
        for bitrate in Bitrate::ALL {
            for mode in Mode::ALL {
                let config = Config { bitrate, mode };
                let bytes = postcard::to_allocvec(&config).unwrap();

                assert_eq!(postcard::from_bytes::<Config>(&bytes).unwrap(), config);
            }
        }
    }

    #[test]
    fn default_config_is_500_kbit_normal() {
        assert_eq!(
            Config::default(),
            Config {
                bitrate: Bitrate::Kbit500,
                mode: Mode::Normal,
            }
        );
    }

    /// A newer host must not have its request misread as some other setting.
    #[test]
    fn config_rejects_unknown_discriminants() {
        assert!(postcard::from_bytes::<Config>(&[4, 0]).is_err());
        assert!(postcard::from_bytes::<Config>(&[0, 3]).is_err());
        assert!(postcard::from_bytes::<Config>(&[2]).is_err());
    }

    #[test]
    fn bitrates_map_to_bits_per_second() {
        let rates: Vec<_> = Bitrate::ALL.map(Bitrate::bits_per_second).into();

        assert_eq!(rates, [125_000, 250_000, 500_000, 1_000_000]);

        for rate in Bitrate::ALL {
            assert_eq!(
                Bitrate::from_bits_per_second(rate.bits_per_second()),
                Some(rate)
            );
        }

        assert_eq!(Bitrate::from_bits_per_second(100_000), None);
        assert_eq!(Bitrate::Mbit1.to_string(), "1000 kbit/s");
    }

    #[test]
    fn parses_versions() {
        assert_eq!(
            FirmwareVersion::parse("1.22.333"),
            Some(FirmwareVersion {
                major: 1,
                minor: 22,
                patch: 333
            })
        );
        assert_eq!(
            FirmwareVersion::parse("0.1.0-rc.1"),
            FirmwareVersion::parse("0.1.0")
        );
        assert_eq!(
            FirmwareVersion::parse("0.1.0+build"),
            FirmwareVersion::parse("0.1.0")
        );
        assert_eq!(
            FirmwareVersion::parse("65535.0.0").map(|v| v.major),
            Some(65535)
        );
    }

    #[test]
    fn rejects_malformed_versions() {
        for version in [
            "",
            "1",
            "1.2",
            "1.2.",
            "1..2",
            ".1.2",
            "1.2.3.4",
            "a.b.c",
            "1.2.x",
            "65536.0.0",
            "1.2-rc.3",
        ] {
            assert_eq!(FirmwareVersion::parse(version), None, "{version}");
        }
    }

    #[test]
    fn this_crates_version_parses() {
        assert!(FirmwareVersion::parse(env!("CARGO_PKG_VERSION")).is_some());
    }

    #[test]
    fn version_displays_as_semver() {
        let version = FirmwareVersion::parse("1.2.3").unwrap();

        assert_eq!(version.to_string(), "1.2.3");
    }
}
