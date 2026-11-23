//! Timeouts the bootloader keeps while it waits on the bus.
//!
//! The bootloader has no clock to read, only a tick every millisecond, so a
//! [`Timeout`] counts ticks rather than comparing timestamps.

/// How long a node waits for the next **Block data** frame before it gives
/// up on the block and answers with [`Error::Timeout`](crate::Error::Timeout).
pub const BLOCK_TIMEOUT_MS: u32 = 500;

/// How long a node tries to queue a frame before it drops it. Only a bus on
/// which nobody acknowledges, or one busy with higher-priority frames for
/// this long, keeps the transmit queue full.
pub const SEND_TIMEOUT_MS: u32 = 100;

// A send inside the block loop takes ticks from the block timeout, so it
// must give up well before the block does.
const _: () = assert!(SEND_TIMEOUT_MS < BLOCK_TIMEOUT_MS);

/// A limit in milliseconds, advanced one tick at a time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timeout {
    limit_ms: u32,
    elapsed_ms: u32,
}

impl Timeout {
    pub const fn new(limit_ms: u32) -> Self {
        Self {
            limit_ms,
            elapsed_ms: 0,
        }
    }

    /// Starts counting from zero again, as when the awaited frame arrives.
    pub fn restart(&mut self) {
        self.elapsed_ms = 0;
    }

    /// One millisecond has passed. Returns whether the timeout has expired,
    /// and keeps returning true until it is restarted.
    pub fn tick(&mut self) -> bool {
        self.elapsed_ms = self.elapsed_ms.saturating_add(1);
        self.expired()
    }

    pub fn expired(&self) -> bool {
        self.elapsed_ms >= self.limit_ms
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ticks_until_expired(timeout: &mut Timeout) -> u32 {
        (1..=u32::MAX).find(|_| timeout.tick()).unwrap()
    }

    #[test]
    fn expires_after_the_limit() {
        let mut timeout = Timeout::new(BLOCK_TIMEOUT_MS);

        assert!(!timeout.expired());
        assert_eq!(ticks_until_expired(&mut timeout), BLOCK_TIMEOUT_MS);
        assert!(timeout.expired());
        assert!(timeout.tick());
    }

    #[test]
    fn restart_counts_from_zero() {
        let mut timeout = Timeout::new(10);

        for _ in 0..9 {
            assert!(!timeout.tick());
        }
        timeout.restart();

        assert_eq!(ticks_until_expired(&mut timeout), 10);
    }

    /// A block arriving one sub-block at a time never times out, however
    /// long it takes in total.
    #[test]
    fn steady_frames_keep_it_alive() {
        let mut timeout = Timeout::new(BLOCK_TIMEOUT_MS);

        for _ in 0..256 {
            for _ in 0..BLOCK_TIMEOUT_MS - 1 {
                assert!(!timeout.tick());
            }
            timeout.restart();
        }
    }

    #[test]
    fn a_zero_limit_has_already_expired() {
        assert!(Timeout::new(0).expired());
    }

    #[test]
    fn ticking_past_the_limit_does_not_wrap() {
        let mut timeout = Timeout {
            limit_ms: u32::MAX,
            elapsed_ms: u32::MAX - 1,
        };

        assert!(timeout.tick());
        assert!(timeout.tick());
    }
}
