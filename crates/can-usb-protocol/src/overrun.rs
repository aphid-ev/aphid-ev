//! Counting received frames that did not fit in the interface's queue, and
//! reporting them as one [`BusEvent::Overrun`].
//!
//! The interface's receive task must not wait for room in its queue to the
//! USB side: while it waits, the controller's small hardware FIFO fills and
//! frames are lost where nothing sees them. So it queues each event with a
//! non-blocking send through [`OverrunCounter::forward`], which counts what
//! does not fit and queues the count ahead of the next event once there is
//! room again.

use crate::BusEvent;

/// Whether [`OverrunCounter::forward`] queued an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Forwarded {
    /// The event is in the queue.
    Queued,
    /// The queue was full and the event is gone. A lost frame is counted;
    /// any other event is only reported by this value.
    Lost,
}

/// The number of received frames lost since the last
/// [`BusEvent::Overrun`] was queued.
///
/// The count saturates at `u16::MAX`, so an overrun of `u16::MAX` frames
/// means at least that many.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct OverrunCounter {
    lost: u16,
    first_lost_us: u32,
}

impl OverrunCounter {
    /// A counter with nothing lost.
    pub const fn new() -> Self {
        Self {
            lost: 0,
            first_lost_us: 0,
        }
    }

    /// Frames lost and not yet reported.
    pub fn lost(&self) -> u16 {
        self.lost
    }

    /// Whether an overrun is waiting to be queued.
    pub fn is_pending(&self) -> bool {
        self.lost > 0
    }

    /// The overrun waiting to be queued, stamped with the time of the first
    /// frame it counts. `None` when nothing was lost.
    pub fn pending(&self) -> Option<BusEvent> {
        self.is_pending().then_some(BusEvent::Overrun {
            timestamp_us: self.first_lost_us,
            lost: self.lost,
        })
    }

    /// Counts `count` frames lost at `timestamp_us`. The overrun keeps the
    /// time of the first frame it counts.
    pub fn record(&mut self, timestamp_us: u32, count: u16) {
        if count == 0 {
            return;
        }
        if self.lost == 0 {
            self.first_lost_us = timestamp_us;
        }
        self.lost = self.lost.saturating_add(count);
    }

    /// Queues the pending overrun, if any, with `try_send`. Returns whether
    /// nothing is pending afterwards.
    pub fn flush<E>(&mut self, mut try_send: impl FnMut(BusEvent) -> Result<(), E>) -> bool {
        match self.pending() {
            None => true,
            Some(overrun) => {
                if try_send(overrun).is_ok() {
                    *self = Self::new();
                    true
                } else {
                    false
                }
            }
        }
    }

    /// Queues `event` with `try_send`, which must not block. The pending
    /// overrun goes first, so the host sees the gap where it happened. When
    /// the overrun does not fit, `event` is not tried: it would land ahead
    /// of the gap it belongs behind.
    pub fn forward<E>(
        &mut self,
        event: BusEvent,
        mut try_send: impl FnMut(BusEvent) -> Result<(), E>,
    ) -> Forwarded {
        if self.flush(&mut try_send) && try_send(event).is_ok() {
            return Forwarded::Queued;
        }
        if let BusEvent::Frame { timestamp_us, .. } = event {
            self.record(timestamp_us, 1);
        }
        Forwarded::Lost
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BusError, Frame};
    use std::collections::VecDeque;
    use std::vec::Vec;

    /// A bounded queue standing in for the firmware's channel.
    struct Queue {
        events: VecDeque<BusEvent>,
        capacity: usize,
    }

    impl Queue {
        fn new(capacity: usize) -> Self {
            Self {
                events: VecDeque::new(),
                capacity,
            }
        }

        fn try_send(&mut self, event: BusEvent) -> Result<(), BusEvent> {
            if self.events.len() < self.capacity {
                self.events.push_back(event);
                Ok(())
            } else {
                Err(event)
            }
        }

        fn drain(&mut self) -> Vec<BusEvent> {
            self.events.drain(..).collect()
        }
    }

    fn frame(timestamp_us: u32) -> BusEvent {
        BusEvent::Frame {
            timestamp_us,
            frame: Frame::new(0x100u16, &[timestamp_us as u8]).unwrap(),
        }
    }

    fn overrun(timestamp_us: u32, lost: u16) -> BusEvent {
        BusEvent::Overrun { timestamp_us, lost }
    }

    #[test]
    fn queues_events_in_order_while_there_is_room() {
        let mut counter = OverrunCounter::new();
        let mut queue = Queue::new(4);

        for t in 0..4 {
            assert_eq!(
                counter.forward(frame(t), |e| queue.try_send(e)),
                Forwarded::Queued
            );
        }

        assert_eq!(queue.drain(), [frame(0), frame(1), frame(2), frame(3)]);
        assert_eq!(counter.pending(), None);
    }

    #[test]
    fn counts_frames_that_do_not_fit() {
        let mut counter = OverrunCounter::new();
        let mut queue = Queue::new(2);

        for t in 0..5 {
            counter.forward(frame(t), |e| queue.try_send(e));
        }

        assert_eq!(counter.lost(), 3);
        assert_eq!(counter.pending(), Some(overrun(2, 3)));
        assert_eq!(queue.drain(), [frame(0), frame(1)]);
    }

    #[test]
    fn the_overrun_is_the_first_event_after_room_frees_up() {
        let mut counter = OverrunCounter::new();
        let mut queue = Queue::new(2);

        for t in 0..4 {
            counter.forward(frame(t), |e| queue.try_send(e));
        }
        assert_eq!(queue.drain(), [frame(0), frame(1)]);

        assert_eq!(
            counter.forward(frame(10), |e| queue.try_send(e)),
            Forwarded::Queued
        );

        assert_eq!(queue.drain(), [overrun(2, 2), frame(10)]);
        assert!(!counter.is_pending());
    }

    #[test]
    fn a_frame_after_an_overrun_that_does_not_fit_is_counted_not_queued() {
        let mut counter = OverrunCounter::new();
        let mut queue = Queue::new(2);

        for t in 0..3 {
            counter.forward(frame(t), |e| queue.try_send(e));
        }
        queue.events.pop_front();

        // One free slot: the overrun takes it, the frame is lost, and the
        // count starts again from that frame.
        assert_eq!(
            counter.forward(frame(5), |e| queue.try_send(e)),
            Forwarded::Lost
        );
        assert_eq!(queue.drain(), [frame(1), overrun(2, 1)]);
        assert_eq!(counter.pending(), Some(overrun(5, 1)));
    }

    #[test]
    fn a_frame_is_not_tried_while_the_overrun_does_not_fit() {
        let mut counter = OverrunCounter::new();
        counter.record(7, 1);
        let mut sent = Vec::new();

        let forwarded = counter.forward(frame(8), |e| {
            sent.push(e);
            Err(e)
        });

        assert_eq!(forwarded, Forwarded::Lost);
        assert_eq!(sent, [overrun(7, 1)]);
        assert_eq!(counter.pending(), Some(overrun(7, 2)));
    }

    #[test]
    fn flush_queues_the_overrun_without_a_new_event() {
        let mut counter = OverrunCounter::new();
        let mut queue = Queue::new(1);

        counter.forward(frame(0), |e| queue.try_send(e));
        counter.forward(frame(1), |e| queue.try_send(e));
        assert!(!counter.flush(|e| queue.try_send(e)));

        queue.drain();
        assert!(counter.flush(|e| queue.try_send(e)));
        assert_eq!(queue.drain(), [overrun(1, 1)]);
        assert!(counter.flush(|e| queue.try_send(e)));
        assert!(queue.drain().is_empty());
    }

    #[test]
    fn a_lost_bus_error_is_reported_but_not_counted() {
        let mut counter = OverrunCounter::new();
        let error = BusEvent::BusError {
            timestamp_us: 3,
            error: BusError::Crc,
        };

        assert_eq!(counter.forward(error, Err), Forwarded::Lost);
        assert!(!counter.is_pending());
    }

    #[test]
    fn the_count_saturates() {
        let mut counter = OverrunCounter::new();

        counter.record(1, u16::MAX - 1);
        counter.record(2, 5);
        counter.forward(frame(3), Err);

        assert_eq!(counter.pending(), Some(overrun(1, u16::MAX)));
    }

    #[test]
    fn recording_nothing_leaves_nothing_pending() {
        let mut counter = OverrunCounter::new();

        counter.record(1, 0);

        assert_eq!(counter.pending(), None);
        counter.record(2, 1);
        assert_eq!(counter.pending(), Some(overrun(2, 1)));
    }
}
