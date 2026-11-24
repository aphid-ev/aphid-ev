//! Sending a queue of values as full bulk packets, and accounting for the
//! values a failed write drops.
//!
//! The interface's write task waits for one [`BusEvent`](crate::BusEvent),
//! then builds a packet with [`Batcher::build`] from that event and
//! everything else already queued, and writes it. The value that did not fit
//! stays in the batcher and opens the next packet, so nothing is lost or
//! reordered. [`DropTracker`] counts the events of packets that could not be
//! written and says when to log, so a disconnected host costs two log lines
//! and not one per event:
//!
//! ```
//! use can_usb_protocol::{BULK_IN_PACKET_LIMIT, Batcher, BusError, BusEvent, DropTracker};
//! use std::collections::VecDeque;
//!
//! let mut queue: VecDeque<BusEvent> = (0..10)
//!     .map(|timestamp_us| BusEvent::BusError { timestamp_us, error: BusError::Crc })
//!     .collect();
//! let mut batcher = Batcher::new(BULK_IN_PACKET_LIMIT);
//! let mut drops = DropTracker::new();
//! let mut packets = Vec::new();
//!
//! loop {
//!     // The firmware waits here for the first event when nothing is carried.
//!     let first = if batcher.has_carry() { None } else { queue.pop_front() };
//!     if first.is_none() && !batcher.has_carry() {
//!         break;
//!     }
//!
//!     let rest = core::iter::from_fn(|| queue.pop_front());
//!     let packet = batcher.build(first.into_iter().chain(rest), |_, _| {});
//!
//!     packets.push(packet.to_vec());
//!     drops.written(); // or drops.failed(batcher.count()) when the write fails
//! }
//!
//! assert_eq!(packets.len(), 1);
//! ```

use crate::{PackError, Packer};
use serde::Serialize;

/// Builds packets from a stream of values, keeping the value that did not
/// fit one packet to open the next.
#[derive(Debug, Clone)]
pub struct Batcher<T> {
    packer: Packer,
    carry: Option<T>,
    count: usize,
}

impl<T: Serialize> Batcher<T> {
    /// A batcher for packets of up to `limit` bytes, capped like
    /// [`Packer::with_limit`].
    pub const fn new(limit: usize) -> Self {
        Self {
            packer: Packer::with_limit(limit),
            carry: None,
            count: 0,
        }
    }

    /// Whether a value is waiting to open the next packet. A sender with one
    /// waiting builds the next packet at once instead of waiting for a new
    /// value.
    pub fn has_carry(&self) -> bool {
        self.carry.is_some()
    }

    /// Builds the next packet: the value carried over from the last packet
    /// if there is one, then values from `values` until one does not fit or
    /// `values` ends. The one that did not fit is kept for the next packet,
    /// and nothing after it is taken from `values`.
    ///
    /// A value that cannot be packed even into an empty packet is passed to
    /// `rejected` and skipped. The packet is empty only when there was
    /// nothing to pack.
    pub fn build(
        &mut self,
        values: impl IntoIterator<Item = T>,
        mut rejected: impl FnMut(T, PackError),
    ) -> &[u8] {
        let mut taken = 0;
        let mut skipped = 0;

        self.packer.clear();
        let values = self.carry.take().into_iter().chain(values);
        let carry = self
            .packer
            .fill(values.inspect(|_| taken += 1), |value, error| {
                skipped += 1;
                rejected(value, error);
            });

        self.count = taken - skipped - usize::from(carry.is_some());
        self.carry = carry;
        self.packer.as_bytes()
    }

    /// The last packet [`build`](Self::build) returned.
    pub fn packet(&self) -> &[u8] {
        self.packer.as_bytes()
    }

    /// How many values the last packet holds.
    pub fn count(&self) -> usize {
        self.count
    }
}

/// What a [`DropTracker`] asks the sender to log.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum DropLog {
    /// The first write that failed since writes last went through. The
    /// sender drops values until one succeeds.
    Started,
    /// A write went through after writes failed, and this many values were
    /// dropped in between. Saturates at `u32::MAX`.
    Resumed { dropped: u32 },
}

/// Counts values dropped with packets that could not be written, so that a
/// disconnect is logged when it starts and when it ends, not per packet.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct DropTracker {
    dropped: Option<u32>,
}

impl DropTracker {
    /// A tracker for a sender whose writes go through.
    pub const fn new() -> Self {
        Self { dropped: None }
    }

    /// A write of a packet with `count` values failed and they are dropped.
    /// Returns [`DropLog::Started`] for the first failure in a row.
    pub fn failed(&mut self, count: usize) -> Option<DropLog> {
        let count = u32::try_from(count).unwrap_or(u32::MAX);

        match self.dropped.as_mut() {
            Some(dropped) => {
                *dropped = dropped.saturating_add(count);
                None
            }
            None => {
                self.dropped = Some(count);
                Some(DropLog::Started)
            }
        }
    }

    /// A write went through. Returns [`DropLog::Resumed`] with the count if
    /// writes failed before it.
    pub fn written(&mut self) -> Option<DropLog> {
        self.dropped
            .take()
            .map(|dropped| DropLog::Resumed { dropped })
    }

    /// Values dropped since writes last went through, or `None` while they
    /// go through.
    pub fn dropped(&self) -> Option<u32> {
        self.dropped
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BULK_IN_PACKET_LIMIT, BusError, BusEvent, Frame, MAX_PACKET_SIZE, unpack};
    use std::collections::VecDeque;

    fn event(timestamp_us: u32) -> BusEvent {
        BusEvent::Frame {
            timestamp_us,
            frame: Frame::new(0x123u16, &[0xaa; 8]).unwrap(),
        }
    }

    fn overrun(timestamp_us: u32, lost: u16) -> BusEvent {
        BusEvent::Overrun { timestamp_us, lost }
    }

    /// A stand-in for the firmware's write task. `queue` is the receive
    /// queue, and `arrivals` adds events to it while a packet is written,
    /// as the CAN task does. `write` says whether each write goes through.
    struct Sender {
        batcher: Batcher<BusEvent>,
        drops: DropTracker,
        queue: VecDeque<BusEvent>,
        written: Vec<Vec<u8>>,
        logs: Vec<DropLog>,
    }

    impl Sender {
        fn new(queue: impl IntoIterator<Item = BusEvent>) -> Self {
            Self {
                batcher: Batcher::new(BULK_IN_PACKET_LIMIT),
                drops: DropTracker::new(),
                queue: queue.into_iter().collect(),
                written: Vec::new(),
                logs: Vec::new(),
            }
        }

        /// One pass of the write task's loop. `false` when the task would
        /// wait for an event.
        fn step(&mut self, write_ok: bool) -> bool {
            let first = if self.batcher.has_carry() {
                None
            } else {
                match self.queue.pop_front() {
                    Some(event) => Some(event),
                    None => return false,
                }
            };

            let queue = &mut self.queue;
            let rest = core::iter::from_fn(|| queue.pop_front());
            let packet = self
                .batcher
                .build(first.into_iter().chain(rest), |_, error| {
                    panic!("rejected: {error}")
                })
                .to_vec();
            assert!(!packet.is_empty());

            let log = if write_ok {
                self.written.push(packet);
                self.drops.written()
            } else {
                self.drops.failed(self.batcher.count())
            };
            self.logs.extend(log);
            true
        }

        fn run(&mut self, write_ok: bool) {
            while self.step(write_ok) {}
        }

        fn received(&self) -> Vec<BusEvent> {
            self.written
                .iter()
                .flat_map(|packet| unpack(packet))
                .collect::<Result<_, _>>()
                .unwrap()
        }
    }

    #[test]
    fn one_queued_event_goes_out_alone() {
        let mut sender = Sender::new([event(1)]);

        assert!(sender.step(true));
        assert!(!sender.step(true));
        assert_eq!(sender.received(), [event(1)]);
        assert!(sender.logs.is_empty());
    }

    #[test]
    fn a_packet_takes_every_queued_event_that_fits() {
        let mut sender = Sender::new((0..3).map(event));

        sender.step(true);

        assert_eq!(sender.written.len(), 1);
        assert_eq!(sender.batcher.count(), 3);
        assert!(sender.queue.is_empty());
        assert!(!sender.batcher.has_carry());
    }

    /// Ten 17-byte events: three per 63-byte packet. The fourth is carried,
    /// and the queue still holds the rest.
    #[test]
    fn the_event_that_does_not_fit_opens_the_next_packet() {
        let mut sender = Sender::new((0..10).map(event));

        sender.step(true);

        assert_eq!(sender.batcher.count(), 3);
        assert!(sender.batcher.has_carry());
        assert_eq!(sender.queue.len(), 6);

        sender.step(true);
        assert_eq!(sender.received(), (0..6).map(event).collect::<Vec<_>>());
    }

    /// A burst larger than one packet: short full packets, then the rest,
    /// in order, with an overrun in its place.
    #[test]
    fn a_burst_is_split_into_short_packets_in_order() {
        let mut events: Vec<_> = (0..40).map(event).collect();
        events.insert(17, overrun(17, 300));
        events.push(BusEvent::BusError {
            timestamp_us: 99,
            error: BusError::Crc,
        });
        let mut sender = Sender::new(events.iter().copied());

        sender.run(true);

        assert!(sender.written.len() > 1);
        assert!(sender.written.iter().all(|p| p.len() < MAX_PACKET_SIZE));
        assert_eq!(sender.received(), events);
    }

    /// Four 16-byte events would fill 64 bytes exactly. The bulk IN limit
    /// moves the fourth to the next packet.
    #[test]
    fn no_packet_is_full_size() {
        let sixteen = BusEvent::Frame {
            timestamp_us: 0,
            frame: Frame::new(0x123u16, &[0; 7]).unwrap(),
        };
        assert_eq!(postcard::to_allocvec(&sixteen).unwrap().len(), 16);
        let mut sender = Sender::new([sixteen; 4]);

        sender.run(true);

        let lengths: Vec<_> = sender.written.iter().map(Vec::len).collect();
        assert_eq!(lengths, [48, 16]);
    }

    /// The carried value comes before a new one even if the caller passes
    /// one while a value is carried, so a misused batcher still loses
    /// nothing.
    #[test]
    fn the_carry_goes_first_whatever_the_caller_passes() {
        let mut batcher = Batcher::new(BULK_IN_PACKET_LIMIT);
        batcher.build((0..4).map(event), |_, _| panic!());
        assert!(batcher.has_carry());

        let packet = batcher.build([event(10)], |_, _| panic!()).to_vec();
        let values: Vec<BusEvent> = unpack(&packet).map(Result::unwrap).collect();

        assert_eq!(values, [event(3), event(10)]);
        assert_eq!(batcher.count(), 2);
        assert!(!batcher.has_carry());
    }

    #[test]
    fn nothing_to_pack_builds_an_empty_packet() {
        let mut batcher = Batcher::<BusEvent>::new(BULK_IN_PACKET_LIMIT);

        assert!(batcher.build([], |_, _| panic!()).is_empty());
        assert_eq!(batcher.count(), 0);
    }

    /// A value too large for any packet is carried while the packet has
    /// values, rejected when it opens the next one, and not counted.
    #[test]
    fn a_value_too_large_for_any_packet_is_rejected_and_skipped() {
        let big = [0u8; 64];
        let small = [1u8; 2];
        let mut batcher = Batcher::<&[u8]>::new(MAX_PACKET_SIZE);
        let mut rejected = Vec::new();

        let packet = batcher
            .build([small.as_slice(), big.as_slice()], |v, e| {
                rejected.push((v.len(), e))
            })
            .to_vec();
        assert_eq!(packet, [2, 1, 1]);
        assert_eq!(batcher.count(), 1);
        assert!(batcher.has_carry());

        let packet = batcher
            .build([small.as_slice()], |v, e| rejected.push((v.len(), e)))
            .to_vec();
        assert_eq!(packet, [2, 1, 1]);
        assert_eq!(batcher.count(), 1);
        assert_eq!(rejected, [(64, PackError::TooLarge)]);
        assert!(!batcher.has_carry());
    }

    #[test]
    fn failed_writes_drop_their_events_and_log_once() {
        let mut sender = Sender::new((0..10).map(event));

        sender.run(false);

        assert!(sender.written.is_empty());
        assert_eq!(sender.logs, [DropLog::Started]);
        assert_eq!(sender.drops.dropped(), Some(10));
    }

    /// A carried event is not dropped with the packet that failed: it opens
    /// the next packet, which may go through.
    #[test]
    fn the_carried_event_survives_a_failed_write() {
        let mut sender = Sender::new((0..4).map(event));

        sender.step(false);
        sender.step(true);

        assert_eq!(sender.received(), [event(3)]);
        assert_eq!(
            sender.logs,
            [DropLog::Started, DropLog::Resumed { dropped: 3 }]
        );
    }

    #[test]
    fn each_disconnect_is_logged_once_at_start_and_once_at_the_end() {
        let mut sender = Sender::new([]);

        for round in 0..2 {
            sender.queue.extend((0..5).map(event));
            sender.run(false);
            sender.queue.push_back(event(100 + round));
            sender.run(true);
        }

        let resumed = DropLog::Resumed { dropped: 5 };
        assert_eq!(
            sender.logs,
            [DropLog::Started, resumed, DropLog::Started, resumed]
        );
        assert_eq!(sender.received(), [event(100), event(101)]);
    }

    #[test]
    fn writes_that_go_through_log_nothing() {
        let mut drops = DropTracker::new();

        assert_eq!(drops.written(), None);
        assert_eq!(drops.written(), None);
        assert_eq!(drops.dropped(), None);
    }

    #[test]
    fn the_drop_count_saturates() {
        let mut drops = DropTracker::new();

        assert_eq!(drops.failed(usize::MAX), Some(DropLog::Started));
        assert_eq!(drops.failed(1), None);
        assert_eq!(drops.failed(u32::MAX as usize), None);
        assert_eq!(
            drops.written(),
            Some(DropLog::Resumed { dropped: u32::MAX })
        );
    }
}
