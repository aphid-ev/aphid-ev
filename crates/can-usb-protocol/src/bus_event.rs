use crate::{BusError, Frame};
use core::fmt;
use serde::{Deserialize, Serialize};

/// Something the interface saw on the bus, sent to the host on
/// [`BULK_IN_EP`](crate::BULK_IN_EP).
///
/// Every event carries `timestamp_us`, the interface's clock in microseconds
/// when it happened. It is a free-running counter that wraps after
/// 2³² µs (about 71.6 minutes); the host unwraps it. It is encoded as four
/// little-endian bytes rather than a varint, so every event of a kind has
/// the same size.
///
/// Encoded as a varint tag (the variant's position) followed by the fields
/// in order. `wire_encoding_is_stable` pins the bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum BusEvent {
    /// A frame received from the bus, or sent by the interface in internal
    /// loopback mode.
    Frame {
        #[serde(with = "postcard::fixint::le")]
        timestamp_us: u32,
        frame: Frame,
    },
    /// The controller detected an error on the bus.
    BusError {
        #[serde(with = "postcard::fixint::le")]
        timestamp_us: u32,
        error: BusError,
    },
    /// The controller's fault confinement state changed, with its transmit
    /// and receive error counters at that moment.
    BusState {
        #[serde(with = "postcard::fixint::le")]
        timestamp_us: u32,
        state: BusState,
        tec: u8,
        rec: u8,
    },
    /// The interface could not forward `lost` received frames to the host,
    /// because its queue was full. Sent once there is room again, in the
    /// position where the frames went missing.
    Overrun {
        #[serde(with = "postcard::fixint::le")]
        timestamp_us: u32,
        lost: u16,
    },
}

impl BusEvent {
    /// The interface's clock when the event happened.
    pub fn timestamp_us(&self) -> u32 {
        match self {
            Self::Frame { timestamp_us, .. }
            | Self::BusError { timestamp_us, .. }
            | Self::BusState { timestamp_us, .. }
            | Self::Overrun { timestamp_us, .. } => *timestamp_us,
        }
    }
}

impl fmt::Display for BusEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:>10} us ", self.timestamp_us())?;

        match self {
            Self::Frame { frame, .. } => write!(f, "Frame({frame})"),
            Self::BusError { error, .. } => write!(f, "Error({error})"),
            Self::BusState {
                state, tec, rec, ..
            } => write!(f, "State({state:?}, TEC={tec}, REC={rec})"),
            Self::Overrun { lost, .. } => write!(f, "Overrun({lost} lost)"),
        }
    }
}

/// Fault confinement state of the CAN controller (ISO 11898-1). Encoded by
/// position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum BusState {
    /// Both error counters below 128: takes part in the bus normally.
    ErrorActive,
    /// An error counter at 128 or above: still communicates, but signals
    /// errors recessively.
    ErrorPassive,
    /// The transmit error counter passed 255: disconnected from the bus.
    BusOff,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FrameId, MAX_PACKET_SIZE};

    fn frame(id: u16) -> Frame {
        Frame::new(id, &[1, 2, 3]).unwrap()
    }

    fn all_kinds() -> [BusEvent; 4] {
        [
            BusEvent::Frame {
                timestamp_us: 0x1234_5678,
                frame: frame(0x123),
            },
            BusEvent::BusError {
                timestamp_us: 1,
                error: BusError::Crc,
            },
            BusEvent::BusState {
                timestamp_us: u32::MAX,
                state: BusState::ErrorPassive,
                tec: 128,
                rec: 7,
            },
            BusEvent::Overrun {
                timestamp_us: 0,
                lost: 300,
            },
        ]
    }

    #[test]
    fn every_kind_round_trips_through_postcard() {
        for event in all_kinds() {
            let bytes = postcard::to_allocvec(&event).unwrap();

            assert_eq!(postcard::from_bytes::<BusEvent>(&bytes).unwrap(), event);
        }
    }

    /// This is the device-to-host half of the USB protocol, so the encoding
    /// is a contract between two separately built binaries.
    #[test]
    fn wire_encoding_is_stable() {
        let [frame, error, state, overrun] =
            all_kinds().map(|e| postcard::to_allocvec(&e).unwrap());

        assert_eq!(
            frame,
            vec![
                0, // BusEvent::Frame
                0x78, 0x56, 0x34, 0x12, // timestamp, u32 little endian
                0, 0xa3, 0x02, 3, 1, 2, 3, // the frame, encoded as a Frame
            ]
        );
        assert_eq!(
            error,
            vec![
                1, // BusEvent::BusError
                1, 0, 0, 0, // timestamp
                5, // BusError::Crc
            ]
        );
        assert_eq!(
            state,
            vec![
                2, // BusEvent::BusState
                0xff, 0xff, 0xff, 0xff, // timestamp
                1,    // BusState::ErrorPassive
                128,  // TEC
                7,    // REC
            ]
        );
        assert_eq!(
            overrun,
            vec![
                3, // BusEvent::Overrun
                0, 0, 0, 0, // timestamp
                0xac, 0x02, // 300 lost, varint
            ]
        );
    }

    #[test]
    fn bus_state_wire_encoding_is_stable() {
        for (tag, state) in [
            BusState::ErrorActive,
            BusState::ErrorPassive,
            BusState::BusOff,
        ]
        .iter()
        .enumerate()
        {
            assert_eq!(postcard::to_allocvec(state).unwrap(), vec![tag as u8]);
        }
    }

    /// An event is never split across packets, so the largest one must fit
    /// in a single packet with room to spare.
    #[test]
    fn largest_event_fits_a_packet() {
        let largest = [
            BusEvent::Frame {
                timestamp_us: u32::MAX,
                frame: Frame::new(FrameId::MAX_EXT_ID, &[0xff; 8]).unwrap(),
            },
            BusEvent::BusState {
                timestamp_us: u32::MAX,
                state: BusState::BusOff,
                tec: u8::MAX,
                rec: u8::MAX,
            },
            BusEvent::Overrun {
                timestamp_us: u32::MAX,
                lost: u16::MAX,
            },
        ];

        for event in largest {
            let len = postcard::to_allocvec(&event).unwrap().len();

            assert!(len <= 20, "{event:?} takes {len} bytes");
            assert!(len < MAX_PACKET_SIZE);
        }
    }

    /// A newer device's event must fail to decode, not be misread as an
    /// older kind.
    #[test]
    fn deserialize_rejects_unknown_discriminants() {
        assert!(postcard::from_bytes::<BusEvent>(&[4, 0, 0, 0, 0]).is_err());
        assert!(postcard::from_bytes::<BusEvent>(&[2, 0, 0, 0, 0, 3, 0, 0]).is_err());
        assert!(postcard::from_bytes::<BusState>(&[3]).is_err());
    }

    #[test]
    fn deserialize_rejects_a_short_timestamp() {
        assert!(postcard::from_bytes::<BusEvent>(&[3, 0, 0, 0]).is_err());
    }

    /// A malformed frame must not become a decodable event either.
    #[test]
    fn deserialize_rejects_frames_with_an_oversized_payload() {
        let bytes =
            postcard::to_allocvec(&(0u8, [0u8; 4], FrameId::Standard(0x123), [0u8; 9].as_slice()))
                .unwrap();

        assert!(postcard::from_bytes::<BusEvent>(&bytes).is_err());
    }

    #[test]
    fn timestamp_reads_every_kind() {
        let stamps: Vec<_> = all_kinds().iter().map(BusEvent::timestamp_us).collect();

        assert_eq!(stamps, [0x1234_5678, 1, u32::MAX, 0]);
    }

    #[test]
    fn display_labels_each_kind() {
        let [frame, error, state, overrun] = all_kinds().map(|e| e.to_string());

        assert_eq!(frame, " 305419896 us Frame(StdId=0x123 [1, 2, 3])");
        assert_eq!(error, "         1 us Error(Crc)");
        assert_eq!(state, "4294967295 us State(ErrorPassive, TEC=128, REC=7)");
        assert_eq!(overrun, "         0 us Overrun(300 lost)");
    }
}
