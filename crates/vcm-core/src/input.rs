use core::ops::RangeInclusive;

#[derive(Debug, PartialEq, Eq)]
pub enum DirectionInput {
    Forward,
    Reverse,
    Contradicting,
}

#[derive(Debug, PartialEq)]
pub enum ThrottleInput {
    Invalid,
    Valid(f32),
}

#[derive(Debug, PartialEq)]
pub struct CriticalInputs {
    forward: bool,
    reverse: bool,
    brake: bool,
    throttle1: f32,
    throttle2: f32,
}

impl CriticalInputs {
    pub fn new(forward: bool, reverse: bool, brake: bool, throttle1: f32, throttle2: f32) -> Self {
        Self {
            forward,
            reverse,
            brake,
            throttle1,
            throttle2,
        }
    }

    pub fn direction(&self) -> Option<DirectionInput> {
        match (self.forward, self.reverse) {
            (false, false) => None,
            (true, false) => Some(DirectionInput::Forward),
            (false, true) => Some(DirectionInput::Reverse),
            (true, true) => Some(DirectionInput::Contradicting),
        }
    }

    pub fn brake(&self) -> bool {
        self.brake
    }

    pub fn throttle(&self) -> ThrottleInput {
        const T_POS: Line = Line::new(1.0, 0.0);
        const T_NEG: Line = Line::new(-1.0, 1.0);
        const MAX_THROTTLE_DEVIATION: f32 = 0.01;
        const VALID: RangeInclusive<f32> = 0.0..=1.0;

        let pos = T_POS.f(self.throttle1);
        let neg = T_NEG.f(self.throttle2);

        // Both sensors in range and agreeing with each other. The prototype
        // negated the range check and accepted out-of-range readings.
        if VALID.contains(&pos)
            && VALID.contains(&neg)
            && (pos - neg).abs() < MAX_THROTTLE_DEVIATION
        {
            let mean = (pos + neg) / 2.0;
            return ThrottleInput::Valid(mean);
        }
        ThrottleInput::Invalid
    }
}

struct Line {
    k: f32,
    m: f32,
}

impl Line {
    const fn new(k: f32, m: f32) -> Self {
        Self { k, m }
    }

    fn f(&self, x: f32) -> f32 {
        self.k * x + self.m
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn throttle(throttle1: f32, throttle2: f32) -> ThrottleInput {
        CriticalInputs::new(false, false, false, throttle1, throttle2).throttle()
    }

    #[test]
    fn direction_follows_the_switches() {
        let direction =
            |forward, reverse| CriticalInputs::new(forward, reverse, false, 0.0, 1.0).direction();

        assert_eq!(direction(false, false), None);
        assert_eq!(direction(true, false), Some(DirectionInput::Forward));
        assert_eq!(direction(false, true), Some(DirectionInput::Reverse));
        assert_eq!(direction(true, true), Some(DirectionInput::Contradicting));
    }

    #[test]
    fn brake_is_passed_through() {
        assert!(CriticalInputs::new(false, false, true, 0.0, 1.0).brake());
        assert!(!CriticalInputs::new(false, false, false, 0.0, 1.0).brake());
    }

    /// The second sensor reads inverted, so 0.25 and 0.75 agree on a
    /// quarter throttle.
    #[test]
    fn agreeing_sensors_give_their_mean() {
        assert_eq!(throttle(0.25, 0.75), ThrottleInput::Valid(0.25));
        assert_eq!(throttle(0.0, 1.0), ThrottleInput::Valid(0.0));
        assert_eq!(throttle(1.0, 0.0), ThrottleInput::Valid(1.0));
    }

    #[test]
    fn disagreeing_sensors_are_invalid() {
        assert_eq!(throttle(0.25, 0.5), ThrottleInput::Invalid);
    }

    /// The prototype accepted exactly these readings.
    #[test]
    fn out_of_range_sensors_are_invalid() {
        assert_eq!(throttle(1.5, -0.5), ThrottleInput::Invalid);
        assert_eq!(throttle(-0.5, 1.5), ThrottleInput::Invalid);
    }
}
