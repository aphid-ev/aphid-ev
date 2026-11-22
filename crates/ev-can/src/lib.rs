//! Messages on the Nissan Leaf EV-CAN bus, the powertrain bus between the
//! VCM, the inverter and the charger.
//!
//! The wiki's `ev-can` page documents what is known about each message.
//! Fields the page marks as unknown are encoded as the first VCM prototype
//! did and are flagged "unverified" below; check them on the bench before
//! relying on them.

// Tests link the standard test harness, so std is only excluded otherwise.
#![cfg_attr(not(test), no_std)]

mod crc;

pub use crc::{crc8, crc8_table, nissan_crc};

/// VCM keep-alive. Unverified.
pub const VCM_KEEPALIVE1_ID: u16 = 0x11a;
/// Torque request, VCM to inverter, 100 Hz.
pub const TORQUE_REQUEST_ID: u16 = 0x1d4;
/// Inverter status, inverter to VCM.
pub const INVERTER_STATUS_ID: u16 = 0x1da;
/// Second VCM keep-alive. Unverified.
pub const VCM_KEEPALIVE2_ID: u16 = 0x50b;
/// Inverter and motor temperatures. Unverified.
pub const INVERTER_TEMPERATURE_ID: u16 = 0x55a;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The frame is shorter than the message needs.
    BadDlc,
    /// The CRC in the last byte does not match.
    BadCrc,
    /// Not a message this crate decodes.
    UnknownFrame,
}

/// A decoded EV-CAN message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Message {
    /// `counter` is 0 to 255. Unverified.
    VcmKeepalive1 { counter: u8 },
    /// Unverified.
    VcmKeepalive2,
    /// `torque` is signed, positive to accelerate and negative to
    /// regenerate. The scale is not known yet. `counter` loops 0 to 3.
    TorqueRequest { torque: i16, counter: u8 },
    /// `millivolt` is the DC link voltage. `current`, `rpm` and `error` are
    /// unverified.
    InverterStatus {
        millivolt: u32,
        current: i16,
        rpm: i16,
        error: u8,
    },
    /// Raw temperature bytes. Unverified.
    InverterTemperature {
        motor_temperature: u8,
        inverter_temperature: u8,
    },
}

/// The payload of an encoded message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Payload {
    data: [u8; 8],
    len: usize,
}

impl Payload {
    pub fn as_slice(&self) -> &[u8] {
        &self.data[..self.len]
    }
}

impl Message {
    pub fn id(&self) -> u16 {
        match self {
            Self::VcmKeepalive1 { .. } => VCM_KEEPALIVE1_ID,
            Self::VcmKeepalive2 => VCM_KEEPALIVE2_ID,
            Self::TorqueRequest { .. } => TORQUE_REQUEST_ID,
            Self::InverterStatus { .. } => INVERTER_STATUS_ID,
            Self::InverterTemperature { .. } => INVERTER_TEMPERATURE_ID,
        }
    }

    /// Encodes the messages the VCM sends. `None` for messages it only
    /// receives.
    pub fn encode(&self) -> Option<Payload> {
        let (data, len) = match *self {
            Self::TorqueRequest { torque, counter } => {
                let [msb, lsb] = torque.to_be_bytes();
                (
                    with_crc([0x6e, 0x6e, msb, lsb, (counter & 0x03) << 6, 0x44, 0x01, 0]),
                    8,
                )
            }
            Self::VcmKeepalive1 { counter } => (
                with_crc([0x4e, 0x40, 0x00, 0xaa, 0xc0, 0x00, counter, 0]),
                8,
            ),
            Self::VcmKeepalive2 => ([0x00, 0x00, 0x06, 0xc0, 0x00, 0x00, 0x00, 0], 7),
            Self::InverterStatus { .. } | Self::InverterTemperature { .. } => return None,
        };

        Some(Payload { data, len })
    }

    /// Decodes the messages the VCM receives, and the torque request, which
    /// the monitor MCU reads back to check what the main MCU sends.
    pub fn decode(id: u16, data: &[u8]) -> Result<Self, Error> {
        match id {
            TORQUE_REQUEST_ID => {
                let data: &[u8; 8] = data.try_into().map_err(|_| Error::BadDlc)?;

                if nissan_crc(&data[..7]) != data[7] {
                    return Err(Error::BadCrc);
                }

                Ok(Self::TorqueRequest {
                    torque: i16::from_be_bytes([data[2], data[3]]),
                    counter: data[4] >> 6,
                })
            }
            INVERTER_STATUS_ID => {
                let data: &[u8; 8] = data.try_into().map_err(|_| Error::BadDlc)?;

                Ok(Self::InverterStatus {
                    millivolt: u16::from_be_bytes([data[0], data[1]]) as u32 * 500,
                    current: i16::from_le_bytes([data[2], data[3]]),
                    rpm: i16::from_le_bytes([data[4], data[5]]),
                    error: data[6],
                })
            }
            INVERTER_TEMPERATURE_ID => match data {
                [motor, inverter, ..] => Ok(Self::InverterTemperature {
                    motor_temperature: *motor,
                    inverter_temperature: *inverter,
                }),
                _ => Err(Error::BadDlc),
            },
            _ => Err(Error::UnknownFrame),
        }
    }

    /// Builds a driver's frame type, such as embassy-stm32's. `None` for
    /// messages the VCM does not send.
    pub fn to_can<F: embedded_can::Frame>(&self) -> Option<F> {
        let id = embedded_can::StandardId::new(self.id())?;

        F::new(id, self.encode()?.as_slice())
    }

    /// Decodes a driver's frame. Extended and remote frames are not EV-CAN
    /// messages.
    pub fn from_can<F: embedded_can::Frame>(frame: &F) -> Result<Self, Error> {
        match frame.id() {
            embedded_can::Id::Standard(id) if !frame.is_remote_frame() => {
                Self::decode(id.as_raw(), frame.data())
            }
            _ => Err(Error::UnknownFrame),
        }
    }
}

fn with_crc(mut data: [u8; 8]) -> [u8; 8] {
    data[7] = nissan_crc(&data[..7]);
    data
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn torque_request_puts_the_msb_in_byte_2() {
        let payload = Message::TorqueRequest {
            torque: 0x0123,
            counter: 2,
        }
        .encode()
        .unwrap();
        let data = payload.as_slice();

        assert_eq!(data.len(), 8);
        assert_eq!(data[2..4], [0x01, 0x23]);
        assert_eq!(data[4], 0x80);
        assert_eq!(data[7], nissan_crc(&data[..7]));
    }

    #[test]
    fn torque_request_round_trips() {
        for torque in [i16::MIN, -1000, 0, 1000, i16::MAX] {
            for counter in 0..4 {
                let message = Message::TorqueRequest { torque, counter };
                let payload = message.encode().unwrap();

                assert_eq!(
                    Message::decode(TORQUE_REQUEST_ID, payload.as_slice()),
                    Ok(message)
                );
            }
        }
    }

    #[test]
    fn torque_request_counter_wraps_at_four() {
        let wrapped = Message::TorqueRequest {
            torque: 0,
            counter: 5,
        };
        let payload = wrapped.encode().unwrap();

        assert_eq!(
            Message::decode(TORQUE_REQUEST_ID, payload.as_slice()),
            Ok(Message::TorqueRequest {
                torque: 0,
                counter: 1
            })
        );
    }

    #[test]
    fn torque_request_rejects_a_bad_crc() {
        let payload = Message::TorqueRequest {
            torque: 100,
            counter: 0,
        }
        .encode()
        .unwrap();
        let mut data = *payload.as_slice().first_chunk::<8>().unwrap();
        data[7] ^= 1;

        assert_eq!(
            Message::decode(TORQUE_REQUEST_ID, &data),
            Err(Error::BadCrc)
        );
    }

    /// The old decoders indexed eight bytes without checking the length,
    /// so a short frame panicked.
    #[test]
    fn short_frames_are_rejected() {
        assert_eq!(
            Message::decode(TORQUE_REQUEST_ID, &[0; 7]),
            Err(Error::BadDlc)
        );
        assert_eq!(
            Message::decode(INVERTER_STATUS_ID, &[0; 3]),
            Err(Error::BadDlc)
        );
        assert_eq!(
            Message::decode(INVERTER_TEMPERATURE_ID, &[0]),
            Err(Error::BadDlc)
        );
    }

    #[test]
    fn inverter_voltage_is_big_endian_in_half_volts() {
        let data = [0x02, 0xd0, 0, 0, 0, 0, 0, 0];

        assert_eq!(
            Message::decode(INVERTER_STATUS_ID, &data),
            Ok(Message::InverterStatus {
                millivolt: 360_000,
                current: 0,
                rpm: 0,
                error: 0,
            })
        );
    }

    #[test]
    fn keepalives_carry_their_crc() {
        let one = Message::VcmKeepalive1 { counter: 3 }.encode().unwrap();
        let two = Message::VcmKeepalive2.encode().unwrap();

        assert_eq!(one.as_slice()[6], 3);
        assert_eq!(one.as_slice()[7], nissan_crc(&one.as_slice()[..7]));
        assert_eq!(two.as_slice().len(), 7);
    }

    #[test]
    fn received_messages_do_not_encode() {
        assert!(
            Message::InverterTemperature {
                motor_temperature: 0,
                inverter_temperature: 0
            }
            .encode()
            .is_none()
        );
    }

    #[test]
    fn unknown_ids_are_reported() {
        assert_eq!(Message::decode(0x123, &[]), Err(Error::UnknownFrame));
    }
}
