use crate::input::{CriticalInputs, DirectionInput};

#[derive(Debug, PartialEq, Eq)]
pub enum DirectionError {
    Overspeed,
    InconsistentInput,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Direction {
    Forward,
    #[default]
    Neutral,
    Reverse,
    Error,
}

#[derive(Debug, Default)]
pub struct DirectionController {
    current: Direction,
}

impl DirectionController {
    const MAX_SHIFTING_SPEED: f32 = 3.0; // m/s

    pub fn current(&self) -> Direction {
        self.current
    }

    pub fn tick(&mut self, speed: f32, input: CriticalInputs) -> Result<Direction, DirectionError> {
        if let Some(input) = input.direction() {
            if speed > Self::MAX_SHIFTING_SPEED {
                return Err(DirectionError::Overspeed);
            }

            self.current = match input {
                DirectionInput::Forward => Direction::Forward,
                DirectionInput::Reverse => Direction::Reverse,
                DirectionInput::Contradicting => return Err(DirectionError::InconsistentInput),
            }
        }

        Ok(self.current)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inputs(forward: bool, reverse: bool) -> CriticalInputs {
        CriticalInputs::new(forward, reverse, false, 0.0, 1.0)
    }

    #[test]
    fn starts_in_neutral_and_holds_without_input() {
        let mut controller = DirectionController::default();

        assert_eq!(
            controller.tick(0.0, inputs(false, false)),
            Ok(Direction::Neutral)
        );
    }

    #[test]
    fn shifts_below_the_speed_limit() {
        let mut controller = DirectionController::default();

        assert_eq!(
            controller.tick(1.0, inputs(true, false)),
            Ok(Direction::Forward)
        );
        assert_eq!(
            controller.tick(0.0, inputs(false, false)),
            Ok(Direction::Forward)
        );
        assert_eq!(
            controller.tick(0.0, inputs(false, true)),
            Ok(Direction::Reverse)
        );
    }

    #[test]
    fn refuses_to_shift_at_speed() {
        let mut controller = DirectionController::default();

        assert_eq!(
            controller.tick(3.5, inputs(false, true)),
            Err(DirectionError::Overspeed)
        );
        assert_eq!(controller.current(), Direction::Neutral);
    }

    #[test]
    fn refuses_contradicting_inputs() {
        let mut controller = DirectionController::default();

        assert_eq!(
            controller.tick(0.0, inputs(true, true)),
            Err(DirectionError::InconsistentInput)
        );
        assert_eq!(controller.current(), Direction::Neutral);
    }
}
