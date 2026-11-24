//! What `can-usb throughput` counts and how it turns that into a rate.

use boot_protocol::{BUS_TEST_ECHO_ID, BUS_TEST_ID};
use can_usb_host::TimedEvent;
use can_usb_protocol::{BusEvent, Frame, FrameId, Mode};

/// The identifier test frames come back on. In internal loopback the
/// interface receives its own frames on [`BUS_TEST_ID`]; on a bus,
/// `can-echo` answers them on [`BUS_TEST_ECHO_ID`].
pub fn echo_id(mode: Mode) -> u16 {
    match mode {
        Mode::InternalLoopback => BUS_TEST_ID,
        Mode::Normal | Mode::ListenOnly => BUS_TEST_ECHO_ID,
    }
}

/// Whether `event` is the answer to a test frame.
pub fn is_echo(event: &BusEvent, echo_id: u16) -> bool {
    matches!(event, BusEvent::Frame { frame, .. } if frame.id() == FrameId::Standard(echo_id))
}

/// The `n`th test frame: eight bytes, the sequence number first, so the
/// receiver can tell whether frames come back in order.
pub fn test_frame(id: u16, n: u32) -> Frame {
    let mut data = [0; 8];
    data[..4].copy_from_slice(&n.to_le_bytes());

    // A standard identifier and eight bytes are always a valid frame.
    Frame::new(id, &data).expect("valid test frame")
}

/// Bits a classic data frame takes on the bus, with the three bits of
/// interframe space after it but without stuff bits, which depend on the
/// content and add up to about a fifth more.
pub fn frame_bits(frame: &Frame) -> u32 {
    let header = match frame.id() {
        FrameId::Standard(_) => 47,
        FrameId::Extended(_) => 67,
    };

    header + 8 * frame.data().len() as u32
}

/// The events of one run.
#[derive(Debug, Default)]
pub struct Tally {
    /// Received frames.
    pub frames: u64,
    /// Bits of the received frames, see [`frame_bits`].
    pub bits: u64,
    /// Frames the interface reported lost in overrun events.
    pub lost: u64,
    pub bus_errors: u64,
    /// Test frames whose sequence number was not the one after the previous.
    pub out_of_sequence: u64,
    /// Microseconds on the interface's clock from the first received frame
    /// to the last.
    pub elapsed_us: u64,
    first_time_us: Option<u64>,
    next_sequence: u32,
    test_id: u16,
}

impl Tally {
    /// A tally that checks the sequence of frames with `test_id`, the
    /// [`echo_id`] of the test frames.
    pub fn new(test_id: u16) -> Self {
        Self {
            test_id,
            ..Self::default()
        }
    }

    pub fn add(&mut self, TimedEvent { time_us, event }: TimedEvent) {
        match event {
            BusEvent::Frame { frame, .. } => {
                self.frames += 1;
                self.bits += u64::from(frame_bits(&frame));

                let first = *self.first_time_us.get_or_insert(time_us);
                self.elapsed_us = time_us.saturating_sub(first);

                if frame.id() == FrameId::Standard(self.test_id)
                    && let Some(sequence) = frame.data().first_chunk::<4>()
                {
                    let sequence = u32::from_le_bytes(*sequence);
                    if sequence != self.next_sequence {
                        self.out_of_sequence += 1;
                    }
                    self.next_sequence = sequence.wrapping_add(1);
                }
            }
            BusEvent::Overrun { lost, .. } => self.lost += u64::from(lost),
            BusEvent::BusError { .. } => self.bus_errors += 1,
            BusEvent::BusState { .. } => {}
        }
    }

    /// Received frames per second on the interface's clock, from the first
    /// frame to the last. `None` with fewer than two frames.
    pub fn frames_per_second(&self) -> Option<f64> {
        (self.frames > 1 && self.elapsed_us > 0)
            .then(|| (self.frames - 1) as f64 * 1e6 / self.elapsed_us as f64)
    }

    /// Share of the bus the received frames took at `bits_per_second`, in
    /// percent, without stuff bits. The first frame's bits fall before the
    /// first timestamp, so they are left out.
    pub fn bus_load_percent(&self, bits_per_second: u32) -> Option<f64> {
        let rate = self.frames_per_second()?;
        let bits_per_frame = self.bits as f64 / self.frames as f64;

        Some(rate * bits_per_frame / f64::from(bits_per_second) * 100.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use can_usb_protocol::BusState;

    const ID: u16 = BUS_TEST_ECHO_ID;

    #[test]
    fn echoes_come_back_on_the_echo_id_except_in_loopback() {
        assert_eq!(echo_id(Mode::InternalLoopback), BUS_TEST_ID);
        assert_eq!(echo_id(Mode::Normal), BUS_TEST_ECHO_ID);
        assert_eq!(echo_id(Mode::ListenOnly), BUS_TEST_ECHO_ID);
    }

    #[test]
    fn only_a_frame_on_the_echo_id_is_an_echo() {
        let on = |id: u16| BusEvent::Frame {
            timestamp_us: 0,
            frame: test_frame(id, 0),
        };

        assert!(is_echo(&on(BUS_TEST_ECHO_ID), BUS_TEST_ECHO_ID));
        assert!(!is_echo(&on(BUS_TEST_ID), BUS_TEST_ECHO_ID));
        assert!(!is_echo(
            &BusEvent::BusError {
                timestamp_us: 0,
                error: can_usb_protocol::BusError::BitDominant,
            },
            BUS_TEST_ECHO_ID
        ));
    }

    fn timed(time_us: u64, event: BusEvent) -> TimedEvent {
        TimedEvent { time_us, event }
    }

    fn received(n: u32, time_us: u64) -> TimedEvent {
        timed(
            time_us,
            BusEvent::Frame {
                timestamp_us: time_us as u32,
                frame: test_frame(ID, n),
            },
        )
    }

    #[test]
    fn frames_take_their_nominal_bits() {
        assert_eq!(frame_bits(&test_frame(ID, 0)), 111);
        assert_eq!(frame_bits(&Frame::new(0x123u16, &[]).unwrap()), 47);
        assert_eq!(
            frame_bits(&Frame::new(0x1234_5678u32, &[0; 8]).unwrap()),
            131
        );
    }

    #[test]
    fn a_saturated_500k_bus_is_full() {
        let mut tally = Tally::new(ID);

        // 111 bits at 500 kbit/s is 222 µs per frame.
        for n in 0..1001 {
            tally.add(received(n, u64::from(n) * 222));
        }

        let rate = tally.frames_per_second().unwrap();
        assert!((rate - 1e6 / 222.0).abs() < 0.01, "{rate}");
        let load = tally.bus_load_percent(500_000).unwrap();
        assert!((load - 100.0).abs() < 0.01, "{load}");
        assert_eq!(tally.out_of_sequence, 0);
    }

    /// The time is unwrapped before it gets here, so a run across the
    /// interface clock's wrap measures as any other.
    #[test]
    fn the_rate_uses_the_unwrapped_time() {
        let mut tally = Tally::new(ID);

        tally.add(received(0, u64::from(u32::MAX) - 99));
        tally.add(received(1, (1 << 32) + 100));

        assert_eq!(tally.elapsed_us, 200);
        assert_eq!(tally.frames_per_second(), Some(5000.0));
    }

    #[test]
    fn counts_losses_errors_and_frames_out_of_sequence() {
        let mut tally = Tally::new(ID);

        tally.add(received(0, 0));
        tally.add(timed(
            1,
            BusEvent::Overrun {
                timestamp_us: 1,
                lost: 3,
            },
        ));
        tally.add(received(4, 2));
        tally.add(received(5, 3));
        tally.add(timed(
            4,
            BusEvent::BusError {
                timestamp_us: 4,
                error: can_usb_protocol::BusError::Acknowledge,
            },
        ));
        tally.add(timed(
            5,
            BusEvent::BusState {
                timestamp_us: 5,
                state: BusState::ErrorActive,
                tec: 0,
                rec: 0,
            },
        ));
        // Another node's frame is counted but not sequence-checked.
        tally.add(timed(
            6,
            BusEvent::Frame {
                timestamp_us: 6,
                frame: Frame::new(0x100u16, &[0xff; 8]).unwrap(),
            },
        ));

        assert_eq!(tally.frames, 4);
        assert_eq!(tally.lost, 3);
        assert_eq!(tally.bus_errors, 1);
        assert_eq!(tally.out_of_sequence, 1);
    }

    #[test]
    fn no_rate_without_two_frames() {
        let mut tally = Tally::new(ID);
        assert_eq!(tally.frames_per_second(), None);

        tally.add(received(0, 0));
        assert_eq!(tally.frames_per_second(), None);
        assert_eq!(tally.bus_load_percent(500_000), None);
    }
}
