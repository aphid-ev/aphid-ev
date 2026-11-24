//! Following the CAN controller's fault confinement state, reporting it to
//! the host, and deciding when to recover from bus-off.
//!
//! The interface's CAN task reads the controller's status every
//! [`POLL_INTERVAL_US`] while the bus runs and passes it to a
//! [`BusMonitor`], which says what to queue for the host and whether to
//! start the bus-off recovery. The task creates a new monitor each time it
//! starts the bus, so the first status after a start is always reported, and
//! a stop drops the monitor with any recovery it was waiting for.
//!
//! The controller does not leave bus-off by itself: the interface turns the
//! controller's automatic recovery off and starts the recovery
//! [`BUS_OFF_RECOVERY_DELAY_US`] after it first sees bus-off. If the
//! controller is still bus-off that long after a recovery was started, it
//! is started again.

use crate::{BusError, BusEvent, BusState};

/// How often the CAN task reads the controller's status, in microseconds.
pub const POLL_INTERVAL_US: u32 = 10_000;

/// How long the interface stays bus-off before it starts the recovery, in
/// microseconds.
pub const BUS_OFF_RECOVERY_DELAY_US: u32 = 100_000;

/// The controller's status at one moment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ControllerStatus {
    /// Fault confinement state.
    pub state: BusState,
    /// Transmit error counter.
    pub tec: u8,
    /// Receive error counter.
    pub rec: u8,
    /// The error the controller detected since its status was last read,
    /// if any. The controller forgets it when the status is read, so each
    /// error is in one status only.
    pub error: Option<BusError>,
}

/// What [`BusMonitor::update`] asks the CAN task to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Update {
    /// A bus error to queue for the host. It goes ahead of `state`.
    pub error: Option<BusEvent>,
    /// A [`BusEvent::BusState`] to queue for the host. Once it is queued,
    /// pass it to [`BusMonitor::queued`]; until then every update offers
    /// the latest state again.
    pub state: Option<BusEvent>,
    /// Start the bus-off recovery: take the controller out of init with its
    /// current configuration, after which it waits for 128 sequences of 11
    /// recessive bits and becomes error active.
    pub recover: bool,
}

/// Follows the controller's status during one run of the bus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct BusMonitor {
    /// The state of the latest status.
    current: Option<BusState>,
    /// The state of the latest event the host was sent.
    queued: Option<BusState>,
    /// When bus-off was first seen, or when the recovery was last started.
    bus_off_since_us: Option<u32>,
}

impl BusMonitor {
    /// A monitor for a bus that has just started. It has reported nothing.
    pub const fn new() -> Self {
        Self {
            current: None,
            queued: None,
            bus_off_since_us: None,
        }
    }

    /// Takes the controller's `status` read at `now_us`, on the protocol's
    /// wrapping microsecond clock.
    pub fn update(&mut self, now_us: u32, status: ControllerStatus) -> Update {
        let was_bus_off = self.current == Some(BusState::BusOff);
        self.current = Some(status.state);

        // While the controller is bus-off it reports a bit 0 error for every
        // 11 recessive bits of the recovery. Those are progress, not errors
        // on the bus, so only the error that came with the bus-off itself
        // and those after the recovery are passed on.
        let error = status
            .error
            .filter(|_| !was_bus_off)
            .map(|error| BusEvent::BusError {
                timestamp_us: now_us,
                error,
            });

        let state = (self.queued != Some(status.state)).then_some(BusEvent::BusState {
            timestamp_us: now_us,
            state: status.state,
            tec: status.tec,
            rec: status.rec,
        });

        let recover = match (status.state, self.bus_off_since_us) {
            (BusState::BusOff, None) => {
                self.bus_off_since_us = Some(now_us);
                false
            }
            (BusState::BusOff, Some(since))
                if now_us.wrapping_sub(since) >= BUS_OFF_RECOVERY_DELAY_US =>
            {
                self.bus_off_since_us = Some(now_us);
                true
            }
            (BusState::BusOff, Some(_)) => false,
            _ => {
                self.bus_off_since_us = None;
                false
            }
        };

        Update {
            error,
            state,
            recover,
        }
    }

    /// Records that the state event from [`Update::state`] is in the queue
    /// to the host. Any other event is ignored.
    pub fn queued(&mut self, event: BusEvent) {
        if let BusEvent::BusState { state, .. } = event {
            self.queued = Some(state);
        }
    }

    /// The state of the latest status, `None` before the first.
    pub fn state(&self) -> Option<BusState> {
        self.current
    }

    /// Whether the ERR LED is lit steadily: while the controller is error
    /// passive or bus-off. Otherwise it only flashes on bus errors.
    pub fn err_led_steady(&self) -> bool {
        matches!(
            self.current,
            Some(BusState::ErrorPassive | BusState::BusOff)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Forwarded, OverrunCounter};
    use std::vec::Vec;

    const MS: u32 = 1000;

    fn status(state: BusState, tec: u8, rec: u8) -> ControllerStatus {
        ControllerStatus {
            state,
            tec,
            rec,
            error: None,
        }
    }

    fn active() -> ControllerStatus {
        status(BusState::ErrorActive, 0, 0)
    }

    fn bus_off() -> ControllerStatus {
        status(BusState::BusOff, 248, 0)
    }

    fn state_event(timestamp_us: u32, state: BusState, tec: u8, rec: u8) -> BusEvent {
        BusEvent::BusState {
            timestamp_us,
            state,
            tec,
            rec,
        }
    }

    fn error_event(timestamp_us: u32, error: BusError) -> BusEvent {
        BusEvent::BusError {
            timestamp_us,
            error,
        }
    }

    /// A CAN task with an event queue that always has room: feeds each
    /// `(time, status)` to the monitor, queues what it says and records
    /// when it asked for a recovery.
    #[derive(Default)]
    struct Run {
        monitor: BusMonitor,
        events: Vec<BusEvent>,
        recoveries: Vec<u32>,
    }

    impl Run {
        fn poll(&mut self, now_us: u32, status: ControllerStatus) {
            let update = self.monitor.update(now_us, status);
            self.events.extend(update.error);
            if let Some(state) = update.state {
                self.events.push(state);
                self.monitor.queued(state);
            }
            if update.recover {
                self.recoveries.push(now_us);
            }
        }

        /// Polls every 10 ms from `from_us` up to but not including
        /// `to_us`, with the same status.
        fn hold(&mut self, from_us: u32, to_us: u32, status: ControllerStatus) {
            let mut now = from_us;
            while now < to_us {
                self.poll(now, status);
                now += POLL_INTERVAL_US;
            }
        }
    }

    #[test]
    fn the_first_status_after_a_start_is_reported() {
        let mut run = Run::default();

        run.poll(5, status(BusState::ErrorActive, 1, 2));

        assert_eq!(run.events, [state_event(5, BusState::ErrorActive, 1, 2)]);
    }

    #[test]
    fn nothing_is_reported_while_the_state_stays_the_same() {
        let mut run = Run::default();

        run.poll(0, active());
        run.poll(10 * MS, status(BusState::ErrorActive, 8, 0));
        run.poll(20 * MS, status(BusState::ErrorActive, 120, 3));

        assert_eq!(run.events, [state_event(0, BusState::ErrorActive, 0, 0)]);
    }

    #[test]
    fn every_transition_is_reported_with_its_counters() {
        let mut run = Run::default();

        run.poll(0, active());
        run.poll(10 * MS, status(BusState::ErrorPassive, 128, 0));
        run.poll(20 * MS, status(BusState::ErrorPassive, 136, 0));
        run.poll(30 * MS, status(BusState::ErrorActive, 127, 0));

        assert_eq!(
            run.events,
            [
                state_event(0, BusState::ErrorActive, 0, 0),
                state_event(10 * MS, BusState::ErrorPassive, 128, 0),
                state_event(30 * MS, BusState::ErrorActive, 127, 0),
            ]
        );
        assert!(run.recoveries.is_empty());
    }

    #[test]
    fn bus_off_is_recovered_from_100_ms_after_it_is_seen() {
        let mut run = Run::default();

        run.poll(0, active());
        run.hold(10 * MS, 110 * MS, bus_off());
        assert!(run.recoveries.is_empty(), "too early: {:?}", run.recoveries);

        run.poll(110 * MS, bus_off());
        assert_eq!(run.recoveries, [110 * MS]);

        // The controller counts 128 × 11 recessive bits, then is active.
        run.poll(120 * MS, bus_off());
        run.poll(130 * MS, active());

        assert_eq!(
            run.events,
            [
                state_event(0, BusState::ErrorActive, 0, 0),
                state_event(10 * MS, BusState::BusOff, 248, 0),
                state_event(130 * MS, BusState::ErrorActive, 0, 0),
            ]
        );
        assert_eq!(run.recoveries, [110 * MS]);
    }

    #[test]
    fn a_bus_that_stays_off_is_recovered_again_every_100_ms_without_new_events() {
        let mut run = Run::default();

        run.hold(0, 400 * MS, bus_off());

        assert_eq!(run.recoveries, [100 * MS, 200 * MS, 300 * MS]);
        assert_eq!(run.events, [state_event(0, BusState::BusOff, 248, 0)]);
    }

    #[test]
    fn a_shorted_bus_gives_one_bus_off_and_one_recovered_event_per_cycle() {
        let mut run = Run::default();
        let mut t = 0;

        for _ in 0..3 {
            run.hold(t, t + 30 * MS, active());
            t += 30 * MS;
            run.hold(t, t + 110 * MS, bus_off());
            t += 110 * MS;
        }
        run.poll(t, active());

        let bus_offs = run
            .events
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    BusEvent::BusState {
                        state: BusState::BusOff,
                        ..
                    }
                )
            })
            .count();
        let actives = run
            .events
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    BusEvent::BusState {
                        state: BusState::ErrorActive,
                        ..
                    }
                )
            })
            .count();
        assert_eq!(bus_offs, 3);
        // The first is the report after the start.
        assert_eq!(actives, 4);
        assert_eq!(run.recoveries, [130 * MS, 270 * MS, 410 * MS]);
    }

    #[test]
    fn listen_only_can_turn_error_passive_but_is_never_recovered() {
        // A listening controller never transmits, so only its receive
        // counter rises.
        let mut run = Run::default();

        run.poll(0, active());
        run.hold(10 * MS, 500 * MS, status(BusState::ErrorPassive, 0, 127));

        assert_eq!(
            run.events,
            [
                state_event(0, BusState::ErrorActive, 0, 0),
                state_event(10 * MS, BusState::ErrorPassive, 0, 127),
            ]
        );
        assert!(run.recoveries.is_empty());
    }

    #[test]
    fn the_recovery_delay_survives_the_clock_wrapping() {
        let mut run = Run::default();
        let start = u32::MAX - 50 * MS;

        run.poll(start, bus_off());
        run.poll(start.wrapping_add(90 * MS), bus_off());
        assert!(run.recoveries.is_empty());
        run.poll(start.wrapping_add(100 * MS), bus_off());

        assert_eq!(run.recoveries, [start.wrapping_add(100 * MS)]);
    }

    #[test]
    fn a_new_monitor_forgets_a_pending_recovery() {
        // The CAN task makes a new monitor each time it starts the bus, so
        // a stop and start during the wait ends it.
        let mut run = Run::default();
        run.hold(0, 90 * MS, bus_off());

        let mut restarted = Run::default();
        restarted.poll(100 * MS, active());
        restarted.poll(110 * MS, bus_off());
        restarted.poll(200 * MS, bus_off());

        assert!(run.recoveries.is_empty());
        assert!(restarted.recoveries.is_empty());
        assert_eq!(
            restarted.events,
            [
                state_event(100 * MS, BusState::ErrorActive, 0, 0),
                state_event(110 * MS, BusState::BusOff, 248, 0),
            ]
        );
    }

    #[test]
    fn a_state_that_did_not_fit_is_offered_again_with_the_latest_counters() {
        let mut monitor = BusMonitor::new();

        let update = monitor.update(0, status(BusState::ErrorPassive, 128, 0));
        assert!(update.state.is_some());
        // Not queued.

        let update = monitor.update(10 * MS, status(BusState::ErrorPassive, 136, 0));
        assert_eq!(
            update.state,
            Some(state_event(10 * MS, BusState::ErrorPassive, 136, 0))
        );
        monitor.queued(update.state.unwrap());

        let update = monitor.update(20 * MS, status(BusState::ErrorPassive, 144, 0));
        assert_eq!(update.state, None);
    }

    #[test]
    fn only_the_latest_state_is_offered_after_missed_transitions() {
        let mut monitor = BusMonitor::new();
        let first = monitor.update(0, active()).state.unwrap();
        monitor.queued(first);

        // The host stops reading; the bus goes off and recovers meanwhile.
        monitor.update(10 * MS, bus_off());
        monitor.update(120 * MS, active());
        let update = monitor.update(130 * MS, active());

        // The host was last told ErrorActive, which is still true.
        assert_eq!(update.state, None);
    }

    #[test]
    fn state_events_go_through_the_overrun_counter() {
        // With a pending overrun, the state event lands behind it, and a
        // full queue leaves the state to be offered again.
        let mut monitor = BusMonitor::new();
        let mut overrun = OverrunCounter::new();
        overrun.record(3, 2);
        let mut queue: Vec<BusEvent> = Vec::new();

        let state = monitor.update(10, active()).state.unwrap();
        assert_eq!(overrun.forward(state, |_| Err(())), Forwarded::Lost);

        let state = monitor.update(20, active()).state.unwrap();
        let forwarded = overrun.forward(state, |e| {
            queue.push(e);
            Ok::<_, ()>(())
        });
        assert_eq!(forwarded, Forwarded::Queued);
        monitor.queued(state);

        assert_eq!(
            queue,
            [
                BusEvent::Overrun {
                    timestamp_us: 3,
                    lost: 2
                },
                state_event(20, BusState::ErrorActive, 0, 0),
            ]
        );
        assert_eq!(monitor.update(30, active()).state, None);
    }

    #[test]
    fn bus_errors_are_passed_on_ahead_of_the_state() {
        let mut monitor = BusMonitor::new();

        let update = monitor.update(
            7,
            ControllerStatus {
                error: Some(BusError::Acknowledge),
                ..status(BusState::ErrorPassive, 128, 0)
            },
        );

        assert_eq!(update.error, Some(error_event(7, BusError::Acknowledge)));
        assert_eq!(
            update.state,
            Some(state_event(7, BusState::ErrorPassive, 128, 0))
        );
    }

    #[test]
    fn recovery_progress_while_bus_off_is_not_a_bus_error() {
        let mut run = Run::default();
        let with = |error, status: ControllerStatus| ControllerStatus {
            error: Some(error),
            ..status
        };

        run.poll(0, active());
        // The error that took the controller off the bus is reported.
        run.poll(10 * MS, with(BusError::BitRecessive, bus_off()));
        // Bit 0 errors of the recovery sequence are not, nor is the last of
        // them still showing when the controller is back.
        run.poll(120 * MS, with(BusError::BitDominant, bus_off()));
        run.poll(130 * MS, with(BusError::BitDominant, active()));
        run.poll(140 * MS, with(BusError::Crc, active()));

        assert_eq!(
            run.events,
            [
                state_event(0, BusState::ErrorActive, 0, 0),
                error_event(10 * MS, BusError::BitRecessive),
                state_event(10 * MS, BusState::BusOff, 248, 0),
                state_event(130 * MS, BusState::ErrorActive, 0, 0),
                error_event(140 * MS, BusError::Crc),
            ]
        );
    }

    #[test]
    fn the_err_led_is_steady_while_passive_or_off() {
        let mut monitor = BusMonitor::new();
        assert!(!monitor.err_led_steady());

        for (state, steady) in [
            (BusState::ErrorActive, false),
            (BusState::ErrorPassive, true),
            (BusState::BusOff, true),
            (BusState::ErrorActive, false),
        ] {
            monitor.update(0, status(state, 0, 0));
            assert_eq!(monitor.err_led_steady(), steady, "{state:?}");
            assert_eq!(monitor.state(), Some(state));
        }
    }
}
