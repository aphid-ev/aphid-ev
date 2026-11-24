//! Packing values into bulk packets and reading them back.
//!
//! A bulk packet holds a whole number of postcard values back to back, and
//! no value spans two packets. Each packet can therefore be decoded on its
//! own, and a corrupt or truncated packet costs only the values in it.
//!
//! The sender fills a [`Packer`] until a value does not fit, sends the
//! packet, clears it and pushes the value again. The receiver iterates a
//! packet with [`unpack`]:
//!
//! ```
//! use can_usb_protocol::{Frame, PackError, Packer, unpack};
//!
//! let frames = [Frame::new(0x123u16, &[1, 2, 3]).unwrap(); 10];
//! let mut packets = Vec::new();
//! let mut packer = Packer::new();
//!
//! for frame in &frames {
//!     if let Err(PackError::Full) = packer.push(frame) {
//!         packets.push(packer.as_bytes().to_vec());
//!         packer.clear();
//!         packer.push(frame).unwrap();
//!     }
//! }
//! packets.push(packer.as_bytes().to_vec());
//!
//! let received: Vec<Frame> = packets
//!     .iter()
//!     .flat_map(|packet| unpack::<Frame>(packet))
//!     .collect::<Result<_, _>>()
//!     .unwrap();
//!
//! assert_eq!(received, frames);
//! ```
//!
//! A sender that drains a queue uses [`Packer::fill`], which stops at the
//! first value that does not fit and hands it back for the next packet, or
//! [`Batcher`](crate::Batcher), which also keeps that value for the next
//! packet.
//!
//! Bulk IN packets are at most [`BULK_IN_PACKET_LIMIT`] bytes, one short of
//! the endpoint's maximum, so that each one ends its USB transfer. See that
//! constant for what the host may rely on.

#[cfg(doc)]
use crate::BULK_IN_PACKET_LIMIT;
use crate::MAX_PACKET_SIZE;
use core::{fmt, marker::PhantomData};
use serde::{Serialize, de::DeserializeOwned};

/// Why a value could not be added to a [`Packer`]. The packer is unchanged
/// in every case.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum PackError {
    /// The value does not fit in the space left. Send the packet, clear it
    /// and push the value again.
    Full,
    /// The value does not fit even in an empty packet. Not possible for the
    /// protocol's own types, which tests keep well below the packet size.
    TooLarge,
    /// The value could not be encoded.
    Encode,
}

impl fmt::Display for PackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Full => f.write_str("packet is full"),
            Self::TooLarge => f.write_str("value is larger than a packet"),
            Self::Encode => f.write_str("value could not be encoded"),
        }
    }
}

impl core::error::Error for PackError {}

/// Builds one bulk packet out of postcard-encoded values.
#[derive(Clone)]
pub struct Packer {
    buffer: [u8; MAX_PACKET_SIZE],
    len: usize,
    limit: usize,
}

impl Packer {
    /// A packer for packets of up to [`MAX_PACKET_SIZE`] bytes.
    pub const fn new() -> Self {
        Self::with_limit(MAX_PACKET_SIZE)
    }

    /// A packer for packets of up to `limit` bytes, capped at
    /// [`MAX_PACKET_SIZE`]. For a sender that must keep packets short of the
    /// maximum, for example so that a packet always ends a transfer.
    pub const fn with_limit(limit: usize) -> Self {
        Self {
            buffer: [0; MAX_PACKET_SIZE],
            len: 0,
            limit: if limit < MAX_PACKET_SIZE {
                limit
            } else {
                MAX_PACKET_SIZE
            },
        }
    }

    /// Appends `value`, or leaves the packet as it was and says why not.
    pub fn push<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), PackError> {
        let free = self
            .buffer
            .get_mut(self.len..self.limit)
            .ok_or(PackError::Encode)?;

        match postcard::to_slice(value, free) {
            Ok(written) => {
                self.len += written.len();
                Ok(())
            }
            Err(postcard::Error::SerializeBufferFull) if self.len == 0 => Err(PackError::TooLarge),
            Err(postcard::Error::SerializeBufferFull) => Err(PackError::Full),
            Err(_) => Err(PackError::Encode),
        }
    }

    /// Appends values from `values` until one does not fit or `values` ends,
    /// and returns the one that did not fit, to start the next packet with.
    ///
    /// Takes no value from `values` after the one it returns, so nothing is
    /// lost or reordered when `values` drains a queue. A value that does not
    /// fit even in an empty packet, or does not encode, is passed to
    /// `rejected` with the reason and skipped.
    pub fn fill<T: Serialize>(
        &mut self,
        values: impl IntoIterator<Item = T>,
        mut rejected: impl FnMut(T, PackError),
    ) -> Option<T> {
        for value in values {
            match self.push(&value) {
                Ok(()) => {}
                Err(PackError::Full) => return Some(value),
                Err(error) => rejected(value, error),
            }
        }

        None
    }

    /// The packet so far, ready to send.
    pub fn as_bytes(&self) -> &[u8] {
        &self.buffer[..self.len]
    }

    /// Empties the packet for the next one.
    pub fn clear(&mut self) {
        self.len = 0;
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The largest packet this packer builds.
    pub fn limit(&self) -> usize {
        self.limit
    }

    /// Bytes left before the packet reaches its limit.
    pub fn remaining(&self) -> usize {
        self.limit - self.len
    }
}

impl Default for Packer {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for Packer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Packer")
            .field("bytes", &self.as_bytes())
            .field("limit", &self.limit)
            .finish()
    }
}

/// A packet that ends in bytes that are not a whole value: corrupt,
/// truncated, or from a peer with a newer protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct UnpackError {
    /// Where in the packet the undecodable value starts.
    pub offset: usize,
    /// How many bytes from there to the end of the packet were discarded.
    pub discarded: usize,
}

impl fmt::Display for UnpackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "undecodable value at byte {} of the packet, {} bytes discarded",
            self.offset, self.discarded
        )
    }
}

impl core::error::Error for UnpackError {}

/// Iterates the values in one packet. An empty packet yields nothing.
///
/// Yields each value in order. If bytes remain that are not a whole value,
/// it yields one [`UnpackError`] and then ends: the values before it are
/// still good, and nothing after it can be trusted.
pub fn unpack<T: DeserializeOwned>(packet: &[u8]) -> Unpacker<'_, T> {
    Unpacker {
        remaining: packet,
        offset: 0,
        value: PhantomData,
    }
}

/// Iterator returned by [`unpack`].
#[derive(Debug, Clone)]
pub struct Unpacker<'a, T> {
    remaining: &'a [u8],
    offset: usize,
    value: PhantomData<fn() -> T>,
}

impl<T: DeserializeOwned> Iterator for Unpacker<'_, T> {
    type Item = Result<T, UnpackError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining.is_empty() {
            return None;
        }

        match postcard::take_from_bytes::<T>(self.remaining) {
            Ok((value, rest)) => {
                self.offset += self.remaining.len() - rest.len();
                self.remaining = rest;
                Some(Ok(value))
            }
            Err(_) => {
                let error = UnpackError {
                    offset: self.offset,
                    discarded: self.remaining.len(),
                };
                self.offset += self.remaining.len();
                self.remaining = &[];
                Some(Err(error))
            }
        }
    }
}

impl<T: DeserializeOwned> core::iter::FusedIterator for Unpacker<'_, T> {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BULK_IN_PACKET_LIMIT, BusError, BusEvent, Frame, FrameId};
    use std::collections::VecDeque;

    fn frame(id: u16, len: usize) -> Frame {
        Frame::new(id, &[0xaa; 8][..len]).unwrap()
    }

    fn event(timestamp_us: u32) -> BusEvent {
        BusEvent::Frame {
            timestamp_us,
            frame: frame(0x123, 8),
        }
    }

    /// Packs `values` into as many packets as they need.
    fn pack_all<T: Serialize>(packer: &mut Packer, values: &[T]) -> Vec<Vec<u8>> {
        let mut packets = Vec::new();

        for value in values {
            if packer.push(value) == Err(PackError::Full) {
                packets.push(packer.as_bytes().to_vec());
                packer.clear();
                packer.push(value).unwrap();
            }
        }

        if !packer.is_empty() {
            packets.push(packer.as_bytes().to_vec());
        }

        packets
    }

    #[test]
    fn values_are_concatenated_encodings() {
        let mut packer = Packer::new();
        let values = [frame(0x123, 3), frame(0x7ff, 0)];

        for value in &values {
            packer.push(value).unwrap();
        }

        let mut expected = postcard::to_allocvec(&values[0]).unwrap();
        expected.extend(postcard::to_allocvec(&values[1]).unwrap());

        assert_eq!(packer.as_bytes(), expected);
        assert_eq!(packer.len(), expected.len());
        assert_eq!(packer.remaining(), MAX_PACKET_SIZE - expected.len());
    }

    #[test]
    fn events_round_trip_through_packets() {
        let events: Vec<_> = (0..100).map(event).collect();
        let packets = pack_all(&mut Packer::new(), &events);

        let received: Vec<BusEvent> = packets
            .iter()
            .flat_map(|packet| unpack(packet))
            .collect::<Result<_, _>>()
            .unwrap();

        assert_eq!(received, events);
        assert!(packets.iter().all(|packet| packet.len() <= MAX_PACKET_SIZE));
    }

    #[test]
    fn frames_round_trip_through_packets() {
        let frames: Vec<_> = (0..50).map(|i| frame(i, (i % 9) as usize)).collect();
        let packets = pack_all(&mut Packer::new(), &frames);

        let received: Vec<Frame> = packets
            .iter()
            .flat_map(|packet| unpack(packet))
            .collect::<Result<_, _>>()
            .unwrap();

        assert_eq!(received, frames);
    }

    /// A 17-byte event: three fit in 64 bytes, the fourth does not.
    #[test]
    fn a_value_that_does_not_fit_leaves_the_packet_unchanged() {
        let mut packer = Packer::new();

        for timestamp in 0..3 {
            packer.push(&event(timestamp)).unwrap();
        }

        let before = packer.as_bytes().to_vec();

        assert_eq!(packer.push(&event(3)), Err(PackError::Full));
        assert_eq!(packer.as_bytes(), before);

        packer.clear();
        assert!(packer.is_empty());
        assert_eq!(packer.push(&event(3)), Ok(()));
    }

    #[test]
    fn a_value_fills_the_packet_exactly() {
        let mut packer = Packer::with_limit(4);

        packer.push(&frame(0x123, 0)).unwrap(); // 4 bytes

        assert_eq!(packer.remaining(), 0);
        assert_eq!(packer.push(&BusError::Crc), Err(PackError::Full));
    }

    #[test]
    fn a_value_larger_than_a_packet_is_too_large() {
        assert_eq!(
            Packer::new().push([0u8; 64].as_slice()),
            Err(PackError::TooLarge)
        );
        assert_eq!(
            Packer::with_limit(3).push(&frame(0x123, 0)),
            Err(PackError::TooLarge)
        );
    }

    #[test]
    fn the_limit_is_capped_at_the_packet_size() {
        assert_eq!(Packer::with_limit(1000).limit(), MAX_PACKET_SIZE);
        assert_eq!(Packer::with_limit(63).limit(), 63);
        assert_eq!(Packer::default().limit(), MAX_PACKET_SIZE);
    }

    #[test]
    fn a_limited_packer_keeps_packets_short() {
        let events: Vec<_> = (0..100).map(event).collect();

        for packet in pack_all(&mut Packer::with_limit(63), &events) {
            assert!(packet.len() <= 63);
        }
    }

    #[test]
    fn a_zero_limit_packer_accepts_nothing() {
        assert_eq!(
            Packer::with_limit(0).push(&BusError::Crc),
            Err(PackError::TooLarge)
        );
    }

    #[test]
    fn fill_packs_every_queued_value_that_fits() {
        let mut queue: VecDeque<_> = (0..3).map(event).collect();
        let mut packer = Packer::with_limit(BULK_IN_PACKET_LIMIT);

        let carry = packer.fill(core::iter::from_fn(|| queue.pop_front()), |_, _| {
            panic!("rejected")
        });

        assert_eq!(carry, None);
        assert!(queue.is_empty());
        let received: Vec<BusEvent> = unpack(packer.as_bytes()).map(Result::unwrap).collect();
        assert_eq!(received, [event(0), event(1), event(2)]);
    }

    /// The value that opens the next packet is the only one taken from the
    /// queue beyond what the packet holds.
    #[test]
    fn fill_stops_at_the_first_value_that_does_not_fit() {
        let mut queue: VecDeque<_> = (0..10).map(event).collect();
        let mut packer = Packer::with_limit(BULK_IN_PACKET_LIMIT);

        let carry = packer.fill(core::iter::from_fn(|| queue.pop_front()), |_, _| {
            panic!("rejected")
        });

        // 17 bytes each: three fit in 63.
        assert_eq!(packer.len(), 51);
        assert_eq!(carry, Some(event(3)));
        assert_eq!(queue.front(), Some(&event(4)));
        assert_eq!(queue.len(), 6);
    }

    #[test]
    fn fill_with_nothing_queued_leaves_the_packet_empty() {
        let mut packer = Packer::new();

        assert_eq!(
            packer.fill(core::iter::empty::<BusEvent>(), |_, _| panic!("rejected")),
            None
        );
        assert!(packer.is_empty());
    }

    /// A value too large for any packet is handed back while the packet
    /// has values in it, like any value that does not fit. Opening the next
    /// packet with it rejects it there, and the values after it still go.
    #[test]
    fn fill_skips_a_value_that_can_never_fit() {
        let big = [0u8; 64];
        let small = [1u8; 2];
        let mut packer = Packer::new();
        let mut rejected = Vec::new();

        let carry = packer.fill([small.as_slice(), big.as_slice()], |value, error| {
            rejected.push((value.len(), error))
        });
        assert_eq!(carry, Some(big.as_slice()));
        assert_eq!(packer.as_bytes(), [2, 1, 1]);

        packer.clear();
        let carry = packer.fill([big.as_slice(), small.as_slice()], |value, error| {
            rejected.push((value.len(), error))
        });
        assert_eq!(carry, None);
        assert_eq!(rejected, [(64, PackError::TooLarge)]);
        assert_eq!(packer.as_bytes(), [2, 1, 1]);
    }

    #[test]
    fn an_empty_packet_unpacks_to_nothing() {
        assert_eq!(unpack::<BusEvent>(&[]).count(), 0);
    }

    #[test]
    fn a_truncated_tail_yields_the_values_before_it_and_one_error() {
        let mut packer = Packer::new();
        packer.push(&event(1)).unwrap();
        packer.push(&event(2)).unwrap();

        let bytes = packer.as_bytes();
        let truncated = &bytes[..bytes.len() - 1];
        let first_len = postcard::to_allocvec(&event(1)).unwrap().len();

        let items: Vec<_> = unpack::<BusEvent>(truncated).collect();

        assert_eq!(
            items,
            [
                Ok(event(1)),
                Err(UnpackError {
                    offset: first_len,
                    discarded: truncated.len() - first_len,
                })
            ]
        );
    }

    #[test]
    fn garbage_yields_one_error_and_ends() {
        let mut unpacker = unpack::<BusEvent>(&[0xff; 10]);

        assert_eq!(
            unpacker.next(),
            Some(Err(UnpackError {
                offset: 0,
                discarded: 10
            }))
        );
        assert_eq!(unpacker.next(), None);
        assert_eq!(unpacker.next(), None);
    }

    /// An event kind a newer device sends must stop decoding rather than
    /// be read as something else.
    #[test]
    fn an_unknown_event_kind_stops_decoding() {
        let mut packet = postcard::to_allocvec(&event(1)).unwrap();
        packet.extend([9, 0, 0, 0, 0]);
        packet.extend(postcard::to_allocvec(&event(2)).unwrap());

        let items: Vec<_> = unpack::<BusEvent>(&packet).collect();

        assert_eq!(items.len(), 2);
        assert_eq!(items[0], Ok(event(1)));
        assert!(items[1].is_err());
    }

    #[test]
    fn an_invalid_frame_stops_decoding() {
        let mut packet = postcard::to_allocvec(&frame(0x123, 1)).unwrap();
        packet.extend(
            postcard::to_allocvec(&(FrameId::Standard(0x800), [0u8; 0].as_slice())).unwrap(),
        );

        let items: Vec<_> = unpack::<Frame>(&packet).collect();

        assert_eq!(items[0], Ok(frame(0x123, 1)));
        assert!(items[1].is_err());
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn errors_describe_the_discarded_bytes() {
        let error = UnpackError {
            offset: 17,
            discarded: 3,
        };

        assert_eq!(
            error.to_string(),
            "undecodable value at byte 17 of the packet, 3 bytes discarded"
        );
        assert_eq!(PackError::Full.to_string(), "packet is full");
    }
}
