use core::cmp::Ordering;
use core::{error, fmt};
use serde::de::{self, SeqAccess, Visitor};
use serde::ser::SerializeTuple;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameError {
    IdOutOfRange,
    TooMuchData,
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl error::Error for FrameError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FrameId {
    Standard(u16),
    Extended(u32),
}

impl FrameId {
    pub const MAX_STD_ID: u16 = 0x7ff;
    pub const MAX_EXT_ID: u32 = 0x1fff_ffff;

    pub fn is_valid(&self) -> bool {
        match self {
            Self::Standard(id) => id <= &Self::MAX_STD_ID,
            Self::Extended(id) => id <= &Self::MAX_EXT_ID,
        }
    }

    pub fn as_raw(&self) -> u32 {
        match self {
            Self::Standard(id) => *id as u32,
            Self::Extended(id) => *id,
        }
    }

    fn is_extended(&self) -> bool {
        matches!(self, Self::Extended(_))
    }
}

impl fmt::Display for FrameId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameId::Standard(id) => write!(f, "StdId={id:#05x}"),
            FrameId::Extended(id) => write!(f, "ExtId={id:#010x}"),
        }
    }
}

#[cfg(feature = "defmt")]
impl defmt::Format for FrameId {
    fn format(&self, fmt: defmt::Formatter) {
        match self {
            FrameId::Standard(id) => defmt::write!(fmt, "StdId={:#05x}", id),
            FrameId::Extended(id) => defmt::write!(fmt, "ExtId={:#010x}", id),
        }
    }
}

impl TryFrom<u16> for FrameId {
    type Error = FrameError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        if value <= Self::MAX_STD_ID {
            Ok(FrameId::Standard(value))
        } else {
            Err(FrameError::IdOutOfRange)
        }
    }
}

impl TryFrom<u32> for FrameId {
    type Error = FrameError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        if value <= Self::MAX_EXT_ID {
            Ok(FrameId::Extended(value))
        } else {
            Err(FrameError::IdOutOfRange)
        }
    }
}

impl From<embedded_can::Id> for FrameId {
    fn from(value: embedded_can::Id) -> Self {
        match value {
            embedded_can::Id::Standard(id) => FrameId::Standard(id.as_raw()),
            embedded_can::Id::Extended(id) => FrameId::Extended(id.as_raw()),
        }
    }
}

impl TryFrom<FrameId> for embedded_can::Id {
    type Error = FrameError;

    fn try_from(value: FrameId) -> Result<Self, Self::Error> {
        match value {
            FrameId::Standard(id) => embedded_can::StandardId::new(id).map(Self::Standard),
            FrameId::Extended(id) => embedded_can::ExtendedId::new(id).map(Self::Extended),
        }
        .ok_or(FrameError::IdOutOfRange)
    }
}

/// Ordered by arbitration priority: the lower identifier wins the bus, and an
/// extended identifier loses to a standard one of the same value because it
/// sends a recessive SRR bit where the standard frame sends a dominant RTR.
/// The tie-break also keeps this ordering consistent with `Eq`.
impl Ord for FrameId {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_raw()
            .cmp(&other.as_raw())
            .then_with(|| self.is_extended().cmp(&other.is_extended()))
    }
}

impl PartialOrd for FrameId {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// A classic CAN frame.
///
/// Invariant: `dlc <= MAX_DATA_LEN`, and `id` is in range for its kind. The
/// fields are private, and the `Deserialize` impl below validates, so this
/// holds for every `Frame` that exists rather than only constructed ones.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Frame {
    id: FrameId,
    dlc: usize,
    data: [u8; 8],
}

/// Encoded as the identifier followed by a length-prefixed payload, so a frame
/// occupies only as many bytes as it carries rather than always padding to a
/// full payload. That roughly doubles how many frames fit in a bulk transfer.
/// `wire_encoding_is_stable` pins the exact bytes.
impl Serialize for Frame {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut frame = serializer.serialize_tuple(2)?;
        frame.serialize_element(&self.id)?;
        frame.serialize_element(self.data())?;
        frame.end()
    }
}

impl<'de> Deserialize<'de> for Frame {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_tuple(2, FrameVisitor)
    }
}

struct FrameVisitor;

impl<'de> Visitor<'de> for FrameVisitor {
    type Value = Frame;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a CAN identifier followed by a payload")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let id: FrameId = seq
            .next_element()?
            .ok_or_else(|| de::Error::invalid_length(0, &self))?;

        if !id.is_valid() {
            return Err(de::Error::custom(FrameError::IdOutOfRange));
        }

        let payload: Payload = seq
            .next_element()?
            .ok_or_else(|| de::Error::invalid_length(1, &self))?;

        Ok(Frame {
            id,
            dlc: payload.len,
            data: payload.data,
        })
    }
}

/// A payload decoded into a fixed buffer. Anything longer than a frame can
/// carry is rejected as it is read, so a peer cannot make us reserve for it.
struct Payload {
    data: [u8; Frame::MAX_DATA_LEN],
    len: usize,
}

impl<'de> Deserialize<'de> for Payload {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(PayloadVisitor)
    }
}

struct PayloadVisitor;

impl<'de> Visitor<'de> for PayloadVisitor {
    type Value = Payload;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "at most {} payload bytes", Frame::MAX_DATA_LEN)
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut data = [0u8; Frame::MAX_DATA_LEN];
        let mut len = 0;

        while let Some(byte) = seq.next_element::<u8>()? {
            if len == Frame::MAX_DATA_LEN {
                return Err(de::Error::custom(FrameError::TooMuchData));
            }

            data[len] = byte;
            len += 1;
        }

        Ok(Payload { data, len })
    }
}

impl Frame {
    pub const MAX_DATA_LEN: usize = 8;

    pub fn new<T>(id: T, data: &[u8]) -> Result<Self, FrameError>
    where
        T: TryInto<FrameId, Error = FrameError>,
    {
        let id = id.try_into()?;

        if data.len() > Self::MAX_DATA_LEN {
            return Err(FrameError::TooMuchData);
        }

        let dlc = data.len();
        let mut inner_data = [0u8; Self::MAX_DATA_LEN];

        inner_data[..dlc].copy_from_slice(data);

        Ok(Self {
            id,
            dlc,
            data: inner_data,
        })
    }

    pub fn id(&self) -> FrameId {
        self.id
    }

    pub fn data(&self) -> &[u8] {
        &self.data[..self.dlc]
    }
}

/// Ordered by CAN identifier only, so the priority channels drain lower
/// (higher priority) identifiers first. Deliberately inconsistent with `Eq`,
/// which also compares the payload.
impl Ord for Frame {
    fn cmp(&self, other: &Self) -> Ordering {
        self.id.cmp(&other.id)
    }
}

impl PartialOrd for Frame {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Frame {
    /// Converts from a driver's frame type, such as embassy-stm32's.
    ///
    /// Remote frames have no counterpart on the wire and come out as data
    /// frames with an empty payload.
    pub fn from_can<F: embedded_can::Frame>(frame: &F) -> Self {
        let dlc = frame.data().len().min(Self::MAX_DATA_LEN);
        let mut data = [0u8; Self::MAX_DATA_LEN];

        data[..dlc].copy_from_slice(&frame.data()[..dlc]);

        Self {
            id: frame.id().into(),
            dlc,
            data,
        }
    }

    /// Converts to a driver's frame type, such as embassy-stm32's. `None`
    /// when the driver cannot represent the frame.
    pub fn to_can<F: embedded_can::Frame>(&self) -> Option<F> {
        let id: embedded_can::Id = self.id.try_into().ok()?;

        F::new(id, self.data())
    }
}

impl fmt::Display for Frame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {:?}", self.id, self.data())
    }
}

#[cfg(feature = "defmt")]
impl defmt::Format for Frame {
    fn format(&self, fmt: defmt::Formatter) {
        defmt::write!(fmt, "{} {}", self.id, self.data())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_rejects_oversized_payload() {
        assert_eq!(
            Frame::new(0x123u16, &[0u8; 9]),
            Err(FrameError::TooMuchData)
        );
    }

    #[test]
    fn new_rejects_out_of_range_id() {
        assert_eq!(Frame::new(0x800u16, &[]), Err(FrameError::IdOutOfRange));
    }

    #[test]
    fn data_is_truncated_to_dlc() {
        let frame = Frame::new(0x123u16, &[1, 2, 3]).unwrap();

        assert_eq!(frame.data(), &[1, 2, 3]);
    }

    #[test]
    fn empty_payload_is_allowed() {
        let frame = Frame::new(0x123u16, &[]).unwrap();

        assert_eq!(frame.data(), &[] as &[u8]);
    }

    #[test]
    fn u16_id_becomes_standard_and_u32_becomes_extended() {
        assert_eq!(
            Frame::new(0x123u16, &[]).unwrap().id(),
            FrameId::Standard(0x123)
        );
        assert_eq!(
            Frame::new(0x12345u32, &[]).unwrap().id(),
            FrameId::Extended(0x12345)
        );
    }

    #[test]
    fn extended_ids_are_range_checked() {
        assert!(FrameId::Extended(FrameId::MAX_EXT_ID).is_valid());
        assert!(!FrameId::Extended(FrameId::MAX_EXT_ID + 1).is_valid());
        assert_eq!(
            Frame::new(FrameId::MAX_EXT_ID + 1, &[]),
            Err(FrameError::IdOutOfRange)
        );
    }

    #[test]
    fn standard_ids_are_range_checked() {
        assert!(FrameId::Standard(FrameId::MAX_STD_ID).is_valid());
        assert!(!FrameId::Standard(FrameId::MAX_STD_ID + 1).is_valid());
    }

    #[test]
    fn as_raw_widens_standard_ids() {
        assert_eq!(FrameId::Standard(0x123).as_raw(), 0x123);
        assert_eq!(FrameId::Extended(0x12345).as_raw(), 0x12345);
    }

    #[test]
    fn round_trips_through_postcard() {
        let frame = Frame::new(0x123u16, &[1, 2, 3, 4, 5, 6, 7, 8]).unwrap();
        let bytes = postcard::to_allocvec(&frame).unwrap();

        assert_eq!(postcard::from_bytes::<Frame>(&bytes).unwrap(), frame);
    }

    #[test]
    fn round_trips_extended_frames() {
        let frame = Frame::new(FrameId::MAX_EXT_ID, &[0xaa]).unwrap();
        let bytes = postcard::to_allocvec(&frame).unwrap();

        assert_eq!(postcard::from_bytes::<Frame>(&bytes).unwrap(), frame);
    }

    #[test]
    fn round_trip_preserves_a_partial_payload() {
        let frame = Frame::new(0x123u16, &[1, 2, 3]).unwrap();
        let bytes = postcard::to_allocvec(&frame).unwrap();
        let decoded: Frame = postcard::from_bytes(&bytes).unwrap();

        assert_eq!(decoded.data(), &[1, 2, 3]);
    }

    /// The host application and the firmware are built and flashed
    /// separately, so this encoding is a contract between two binaries that
    /// can be different versions. Changing it means changing both sides.
    #[test]
    fn wire_encoding_is_stable() {
        let standard = Frame::new(0x123u16, &[1, 2, 3]).unwrap();

        assert_eq!(
            postcard::to_allocvec(&standard).unwrap(),
            vec![
                0, // FrameId::Standard
                0xa3, 0x02, // id 0x123 as a varint
                3,    // payload length
                1, 2, 3,
            ]
        );

        let extended = Frame::new(0x12345u32, &[0xaa]).unwrap();

        assert_eq!(
            postcard::to_allocvec(&extended).unwrap(),
            vec![
                1, // FrameId::Extended
                0xc5, 0xc6, 0x04, // id 0x12345 as a varint
                1,    // payload length
                0xaa,
            ]
        );
    }

    /// Only the bytes a frame actually carries are transmitted, so more
    /// frames fit in a bulk transfer.
    #[test]
    fn encoded_length_follows_the_payload() {
        let empty = postcard::to_allocvec(&Frame::new(0x123u16, &[]).unwrap()).unwrap();
        let full = postcard::to_allocvec(&Frame::new(0x123u16, &[0; 8]).unwrap()).unwrap();

        assert_eq!(empty.len(), 4);
        assert_eq!(full.len(), 12);
    }

    #[test]
    fn deserialize_rejects_truncated_input() {
        let frame = Frame::new(0x123u16, &[1, 2, 3]).unwrap();
        let bytes = postcard::to_allocvec(&frame).unwrap();

        assert!(postcard::from_bytes::<Frame>(&bytes[..bytes.len() - 1]).is_err());
        assert!(postcard::from_bytes::<Frame>(&[]).is_err());
    }

    /// A peer claiming a payload longer than a frame can carry used to
    /// produce a frame whose `data()` panicked when sliced.
    #[test]
    fn deserialize_rejects_oversized_payload() {
        let bytes =
            postcard::to_allocvec(&(FrameId::Standard(0x123), [0u8; 9].as_slice())).unwrap();

        assert!(postcard::from_bytes::<Frame>(&bytes).is_err());
    }

    /// A length prefix far beyond the buffer must fail rather than be
    /// believed, since the payload is read into a fixed buffer.
    #[test]
    fn deserialize_rejects_an_absurd_length_prefix() {
        let mut bytes = postcard::to_allocvec(&FrameId::Standard(0x123)).unwrap();
        bytes.extend_from_slice(&[0xff, 0xff, 0xff, 0xff, 0x0f]);

        assert!(postcard::from_bytes::<Frame>(&bytes).is_err());
    }

    #[test]
    fn deserialize_rejects_out_of_range_id() {
        let bytes =
            postcard::to_allocvec(&(FrameId::Standard(0x800), [0u8; 0].as_slice())).unwrap();

        assert!(postcard::from_bytes::<Frame>(&bytes).is_err());
    }

    #[test]
    fn ordering_follows_can_priority() {
        let high = Frame::new(0x001u16, &[]).unwrap();
        let low = Frame::new(0x7ffu16, &[]).unwrap();

        assert!(high < low);
    }

    #[test]
    fn ordering_ignores_the_payload() {
        let a = Frame::new(0x123u16, &[1]).unwrap();
        let b = Frame::new(0x123u16, &[2]).unwrap();

        // Deliberate: the priority channels order by identifier only, so
        // these tie for ordering while remaining unequal.
        assert_eq!(a.cmp(&b), Ordering::Equal);
        assert_ne!(a, b);
    }

    /// A standard identifier wins arbitration against an extended one of the
    /// same value, and they are not equal, so they must not compare equal.
    #[test]
    fn standard_ids_outrank_extended_ids_of_the_same_value() {
        let standard = FrameId::Standard(0x100);
        let extended = FrameId::Extended(0x100);

        assert!(standard < extended);
        assert_ne!(standard, extended);
    }

    #[test]
    fn identifier_ordering_agrees_with_equality() {
        let ids = [
            FrameId::Standard(0),
            FrameId::Standard(0x100),
            FrameId::Standard(FrameId::MAX_STD_ID),
            FrameId::Extended(0),
            FrameId::Extended(0x100),
            FrameId::Extended(FrameId::MAX_EXT_ID),
        ];

        for left in ids {
            for right in ids {
                assert_eq!(left.cmp(&right) == Ordering::Equal, left == right);
            }
        }
    }

    /// Stands in for a driver's frame type.
    #[derive(Debug, PartialEq)]
    struct DriverFrame {
        id: embedded_can::Id,
        data: Vec<u8>,
    }

    impl embedded_can::Frame for DriverFrame {
        fn new(id: impl Into<embedded_can::Id>, data: &[u8]) -> Option<Self> {
            (data.len() <= 8).then(|| Self {
                id: id.into(),
                data: data.to_vec(),
            })
        }

        fn new_remote(_id: impl Into<embedded_can::Id>, _dlc: usize) -> Option<Self> {
            None
        }

        fn is_extended(&self) -> bool {
            matches!(self.id, embedded_can::Id::Extended(_))
        }

        fn is_remote_frame(&self) -> bool {
            false
        }

        fn id(&self) -> embedded_can::Id {
            self.id
        }

        fn dlc(&self) -> usize {
            self.data.len()
        }

        fn data(&self) -> &[u8] {
            &self.data
        }
    }

    #[test]
    fn converts_to_and_from_a_driver_frame() {
        for frame in [
            Frame::new(0x123u16, &[1, 2, 3]).unwrap(),
            Frame::new(FrameId::MAX_EXT_ID, &[0; 8]).unwrap(),
            Frame::new(0u16, &[]).unwrap(),
        ] {
            let driver: DriverFrame = frame.to_can().unwrap();

            assert_eq!(Frame::from_can(&driver), frame);
        }
    }

    #[test]
    fn driver_frames_keep_the_identifier_kind() {
        let standard: DriverFrame = Frame::new(0x100u16, &[]).unwrap().to_can().unwrap();
        let extended: DriverFrame = Frame::new(0x100u32, &[]).unwrap().to_can().unwrap();

        assert!(matches!(standard.id, embedded_can::Id::Standard(_)));
        assert!(matches!(extended.id, embedded_can::Id::Extended(_)));
    }

    #[test]
    fn display_renders_ids_in_hex() {
        let standard = Frame::new(0x123u16, &[1, 2, 3]).unwrap();
        let extended = Frame::new(0x12345u32, &[]).unwrap();

        assert_eq!(standard.to_string(), "StdId=0x123 [1, 2, 3]");
        assert_eq!(extended.to_string(), "ExtId=0x00012345 []");
    }
}
