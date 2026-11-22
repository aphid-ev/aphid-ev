use crate::{BusError, Frame};
use core::cmp::Ordering;
use core::fmt;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum BusEvent {
    Frame(Frame),
    BusError(BusError),
}

/// Errors sort before frames so the priority channels surface them first;
/// frames themselves are ordered by CAN identifier (see [`Frame`]).
impl Ord for BusEvent {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Self::Frame(self_frame), Self::Frame(other_frame)) => self_frame.cmp(other_frame),
            (Self::BusError(_), Self::BusError(_)) => Ordering::Equal,
            (Self::Frame(_), Self::BusError(_)) => Ordering::Greater,
            (Self::BusError(_), Self::Frame(_)) => Ordering::Less,
        }
    }
}

impl PartialOrd for BusEvent {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for BusEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Frame(frame) => write!(f, "Frame({frame})"),
            Self::BusError(error) => write!(f, "Error({error})"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FrameId;

    fn frame(id: u16) -> Frame {
        Frame::new(id, &[1, 2, 3]).unwrap()
    }

    #[test]
    fn round_trips_frames_through_postcard() {
        let event = BusEvent::Frame(frame(0x123));
        let bytes = postcard::to_allocvec(&event).unwrap();

        assert_eq!(postcard::from_bytes::<BusEvent>(&bytes).unwrap(), event);
    }

    #[test]
    fn round_trips_errors_through_postcard() {
        let event = BusEvent::BusError(BusError::Crc);
        let bytes = postcard::to_allocvec(&event).unwrap();

        assert_eq!(postcard::from_bytes::<BusEvent>(&bytes).unwrap(), event);
    }

    /// This is the device-to-host half of the USB protocol, so the encoding
    /// is a contract between two separately built binaries.
    #[test]
    fn wire_encoding_is_stable() {
        assert_eq!(
            postcard::to_allocvec(&BusEvent::BusError(BusError::Crc)).unwrap(),
            vec![1, 5] // BusEvent::BusError, then BusError::Crc
        );

        // The frame variant is the frame encoding behind a 0 tag.
        let event = BusEvent::Frame(frame(0x123));
        let mut expected = vec![0];
        expected.extend(postcard::to_allocvec(&frame(0x123)).unwrap());

        assert_eq!(postcard::to_allocvec(&event).unwrap(), expected);
    }

    /// A malformed frame must not become a decodable event either.
    #[test]
    fn deserialize_rejects_frames_with_an_oversized_payload() {
        let bytes =
            postcard::to_allocvec(&(0u32, FrameId::Standard(0x123), [0u8; 9].as_slice())).unwrap();

        assert!(postcard::from_bytes::<BusEvent>(&bytes).is_err());
    }

    #[test]
    fn errors_take_priority_over_frames() {
        let error = BusEvent::BusError(BusError::Crc);
        let frame = BusEvent::Frame(frame(0x001));

        assert!(error < frame);
    }

    #[test]
    fn frames_are_ordered_by_identifier() {
        assert!(BusEvent::Frame(frame(0x001)) < BusEvent::Frame(frame(0x7ff)));
    }

    #[test]
    fn errors_tie_with_each_other() {
        let stuff = BusEvent::BusError(BusError::Stuff);
        let crc = BusEvent::BusError(BusError::Crc);

        assert_eq!(stuff.cmp(&crc), Ordering::Equal);
    }

    #[test]
    fn display_labels_each_variant() {
        assert_eq!(
            BusEvent::Frame(frame(0x123)).to_string(),
            "Frame(StdId=0x123 [1, 2, 3])"
        );
        assert_eq!(BusEvent::BusError(BusError::Crc).to_string(), "Error(Crc)");
    }
}
