//! Turns the interface's wrapping 32-bit timestamps into a 64-bit time.
//!
//! Every [`BusEvent`](crate::BusEvent) carries the interface's clock in
//! microseconds as a `u32`, which wraps after 2³² µs, about 71.6 minutes.
//! The interface sends events in the order they happened, so a timestamp
//! smaller than the one before it means the clock wrapped in between. A
//! [`TimestampUnwrapper`] fed every event in the order received counts those
//! wraps and gives a `u64` that only goes up.
//!
//! It cannot see a wrap it has no event for: between two events more than
//! 2³² µs apart it comes out short by a whole number of 2³² µs.

/// Microseconds in one period of the interface's clock.
pub const TIMESTAMP_PERIOD_US: u64 = 1 << 32;

/// Unwraps the interface's 32-bit timestamps, see the [module](self).
///
/// The time it gives is the interface's clock since it last restarted,
/// counted from the first timestamp it was fed: the first event keeps its
/// 32-bit value, and the wraps after it add [`TIMESTAMP_PERIOD_US`] each.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TimestampUnwrapper {
    /// The last timestamp fed, `None` before the first.
    last: Option<u32>,
    /// Wraps seen so far.
    wraps: u64,
}

impl TimestampUnwrapper {
    pub const fn new() -> Self {
        Self {
            last: None,
            wraps: 0,
        }
    }

    /// The 64-bit time of `timestamp_us`, the timestamp of the next event in
    /// the order the interface sent them.
    pub fn extend(&mut self, timestamp_us: u32) -> u64 {
        if self.last.is_some_and(|last| timestamp_us < last) {
            self.wraps += 1;
        }
        self.last = Some(timestamp_us);

        self.wraps * TIMESTAMP_PERIOD_US + u64::from(timestamp_us)
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

    /// A quiet bus: the next event comes almost a whole period later, at a
    /// smaller value.
    #[test]
    fn a_quiet_gap_shorter_than_a_period_is_measured_exactly() {
        let mut unwrapper = TimestampUnwrapper::new();

        assert_eq!(unwrapper.extend(1000), 1000);
        assert_eq!(unwrapper.extend(999), 999 + TIMESTAMP_PERIOD_US);
    }

    /// The limit, as documented: a gap of a period or more loses whole
    /// periods.
    #[test]
    fn a_quiet_gap_of_a_period_or_more_loses_whole_periods() {
        let mut unwrapper = TimestampUnwrapper::new();

        assert_eq!(unwrapper.extend(1000), 1000);
        // Really 1000 + 2³² + 1 µs later.
        assert_eq!(unwrapper.extend(1001), 1001);
    }

    #[test]
    fn the_same_timestamp_twice_is_not_a_wrap() {
        let mut unwrapper = TimestampUnwrapper::new();

        unwrapper.extend(u32::MAX);
        assert_eq!(unwrapper.extend(u32::MAX), 0xffff_ffff);
    }
}
