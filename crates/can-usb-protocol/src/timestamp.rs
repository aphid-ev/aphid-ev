//! Turns the interface's wrapping 32-bit timestamps into a 64-bit time.
//!
//! Every [`BusEvent`](crate::BusEvent) carries the interface's clock in
//! microseconds as a `u32`, which wraps after 2³² µs, about 71.6 minutes.
//! A [`TimestampUnwrapper`] fed every event in the order received gives a
//! `u64` that does not wrap.
//!
//! Events arrive in the order the interface queued them, but their
//! timestamps can step back a little. A bus error or a bus state is stamped
//! when the firmware reads it, while a received frame's stamp is corrected
//! back towards its arrival on the bus, so a frame queued right after a
//! state event can carry a smaller stamp. The unwrapper therefore reads the
//! difference to the latest timestamp as signed: less than 2³¹ µs forward
//! is time going on, across a wrap or not, and anything else is a step
//! back. A step back gives the earlier time and leaves the count of wraps
//! alone.
//!
//! The limit follows from it: after a gap of 2³¹ µs (about 35.8 minutes)
//! or more between two events, the unwrapper cannot tell the gap from a
//! step back. The events after the gap come out one or more whole periods
//! too early.

/// Microseconds in one period of the interface's clock.
pub const TIMESTAMP_PERIOD_US: u64 = 1 << 32;

/// The longest gap between two events that the unwrapper measures
/// correctly, and the longest step back it recognises as one.
pub const TIMESTAMP_MAX_GAP_US: u64 = 1 << 31;

/// Unwraps the interface's 32-bit timestamps, see the [module](self).
///
/// The time it gives is the interface's clock since it last restarted,
/// counted from the first timestamp it was fed: the first event keeps its
/// 32-bit value, and the wraps after it add [`TIMESTAMP_PERIOD_US`] each.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TimestampUnwrapper {
    /// The latest time given so far, `None` before the first timestamp.
    /// Its low 32 bits are the latest timestamp.
    latest: Option<u64>,
}

impl TimestampUnwrapper {
    pub const fn new() -> Self {
        Self { latest: None }
    }

    /// The 64-bit time of `timestamp_us`, the timestamp of the next event in
    /// the order the interface sent them.
    ///
    /// A step back before the first timestamp fed gives 0.
    pub fn extend(&mut self, timestamp_us: u32) -> u64 {
        let Some(latest) = self.latest else {
            self.latest = Some(u64::from(timestamp_us));
            return u64::from(timestamp_us);
        };

        let forward = u64::from(timestamp_us.wrapping_sub(latest as u32));

        if forward < TIMESTAMP_MAX_GAP_US {
            let time = latest + forward;
            self.latest = Some(time);
            time
        } else {
            latest.saturating_sub(TIMESTAMP_PERIOD_US - forward)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_timestamp_keeps_its_value() {
        assert_eq!(TimestampUnwrapper::new().extend(1234), 1234);
        assert_eq!(TimestampUnwrapper::new().extend(u32::MAX), 0xffff_ffff);
    }

    #[test]
    fn timestamps_that_count_up_stay_as_they_are() {
        let mut unwrapper = TimestampUnwrapper::new();

        let times: Vec<_> = [0, 1, 1, 222, 1_000_000]
            .into_iter()
            .map(|t| unwrapper.extend(t))
            .collect();

        assert_eq!(times, [0, 1, 1, 222, 1_000_000]);
    }

    #[test]
    fn a_wrap_adds_one_period() {
        let mut unwrapper = TimestampUnwrapper::new();

        assert_eq!(unwrapper.extend(u32::MAX - 99), 0xffff_ff9c);
        assert_eq!(unwrapper.extend(100), TIMESTAMP_PERIOD_US + 100);
        assert_eq!(unwrapper.extend(200), TIMESTAMP_PERIOD_US + 200);
    }

    #[test]
    fn every_wrap_counts() {
        let mut unwrapper = TimestampUnwrapper::new();
        let step = 1u32 << 30;
        let mut expected = 0;

        // Four events per period, for ten periods.
        for _ in 0..40 {
            assert_eq!(unwrapper.extend(expected as u32), expected);
            expected += u64::from(step);
        }

        assert_eq!(unwrapper.extend(0), 10 * TIMESTAMP_PERIOD_US);
    }

    /// A frame stamped a few µs before the state event queued ahead of it.
    #[test]
    fn a_step_back_of_a_few_microseconds_is_not_a_wrap() {
        let mut unwrapper = TimestampUnwrapper::new();

        assert_eq!(unwrapper.extend(1000), 1000);
        assert_eq!(unwrapper.extend(997), 997);
        assert_eq!(unwrapper.extend(1001), 1001);
    }

    #[test]
    fn a_step_back_of_a_few_milliseconds_is_not_a_wrap() {
        let mut unwrapper = TimestampUnwrapper::new();

        assert_eq!(unwrapper.extend(10_000_000), 10_000_000);
        assert_eq!(unwrapper.extend(9_995_000), 9_995_000);
        assert_eq!(unwrapper.extend(9_996_000), 9_996_000);
        assert_eq!(unwrapper.extend(10_000_100), 10_000_100);
    }

    /// The event before the step back came after the wrap, the one after
    /// it from before.
    #[test]
    fn a_step_back_across_a_wrap_goes_back_before_it() {
        let mut unwrapper = TimestampUnwrapper::new();

        assert_eq!(unwrapper.extend(u32::MAX - 10), 0xffff_fff5);
        assert_eq!(unwrapper.extend(2), TIMESTAMP_PERIOD_US + 2);
        assert_eq!(unwrapper.extend(u32::MAX - 2), 0xffff_fffd);
        assert_eq!(unwrapper.extend(5), TIMESTAMP_PERIOD_US + 5);
    }

    /// A step back across a wrap the unwrapper has not seen yet comes out
    /// after the wrap would have been.
    #[test]
    fn a_step_back_to_before_a_wrap_and_forward_again() {
        let mut unwrapper = TimestampUnwrapper::new();

        assert_eq!(unwrapper.extend(u32::MAX - 1), 0xffff_fffe);
        assert_eq!(
            unwrapper.extend(u32::MAX - 4000),
            u64::from(u32::MAX - 4000)
        );
        assert_eq!(unwrapper.extend(3), TIMESTAMP_PERIOD_US + 3);
    }

    #[test]
    fn a_step_back_before_the_first_timestamp_stops_at_zero() {
        let mut unwrapper = TimestampUnwrapper::new();

        assert_eq!(unwrapper.extend(1), 1);
        assert_eq!(unwrapper.extend(u32::MAX), 0);
        assert_eq!(unwrapper.extend(2), 2);
    }

    /// A quiet bus: the next event comes almost half a period later.
    #[test]
    fn a_quiet_gap_shorter_than_the_limit_is_measured_exactly() {
        let mut unwrapper = TimestampUnwrapper::new();
        let gap = TIMESTAMP_MAX_GAP_US - 1;

        assert_eq!(unwrapper.extend(u32::MAX - 999), 0xffff_fc18);
        assert_eq!(
            unwrapper.extend((u32::MAX - 999).wrapping_add(gap as u32)),
            0xffff_fc18 + gap
        );
    }

    /// The limit, as documented: a gap of half a period or more reads as a
    /// step back.
    #[test]
    fn a_quiet_gap_at_the_limit_reads_as_a_step_back() {
        let mut unwrapper = TimestampUnwrapper::new();
        let start = 1u32 << 31;

        assert_eq!(unwrapper.extend(start), u64::from(start));
        // Really 2³¹ µs later.
        assert_eq!(unwrapper.extend(0), 0);
    }

    #[test]
    fn the_same_timestamp_twice_is_not_a_wrap() {
        let mut unwrapper = TimestampUnwrapper::new();

        unwrapper.extend(u32::MAX);
        assert_eq!(unwrapper.extend(u32::MAX), 0xffff_ffff);
    }
}
