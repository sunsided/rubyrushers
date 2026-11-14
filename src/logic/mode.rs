//! The ways to play.

/// Seconds on the clock at the start of a timed game.
pub const RUSH_SECS: f32 = 60.0;
/// Seconds a time gem adds.
pub const TIME_GEM_SECS: f32 = 5.0;
/// One in this many new gems carries a time bonus in the timed modes.
pub const TIME_GEM_ODDS: u32 = 45;
/// Seconds between gravity turns in the tilt mode.
pub const TURN_SECS: f32 = 15.0;
/// Seconds before a turn that the player is warned.
pub const WARN_SECS: f32 = 2.5;

/// The game mode and its state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mode {
    /// Score as much as possible before the clock runs out; time gems add to it. A board
    /// without moves is reshuffled.
    Rush {
        /// Seconds left.
        remaining: f32,
    },
    /// No clock; the game ends when no move is left.
    Endless,
    /// Like [`Mode::Rush`], but gravity turns a quarter clockwise every [`TURN_SECS`].
    Tilt {
        /// Seconds left.
        remaining: f32,
        /// Seconds until gravity turns (0: as soon as the board rests).
        turn_in: f32,
    },
}

impl Mode {
    /// A fresh timed game.
    pub const fn rush() -> Self {
        Self::Rush {
            remaining: RUSH_SECS,
        }
    }

    /// A fresh tilt game.
    pub const fn tilt() -> Self {
        Self::Tilt {
            remaining: RUSH_SECS,
            turn_in: TURN_SECS,
        }
    }

    /// Runs the clocks; true once the game clock has run out.
    pub fn tick(&mut self, dt: f32) -> bool {
        if let Self::Tilt { turn_in, .. } = self {
            *turn_in = (*turn_in - dt).max(0.0);
        }
        match self.remaining_mut() {
            Some(remaining) => {
                *remaining = (*remaining - dt).max(0.0);
                *remaining <= 0.0
            }
            None => false,
        }
    }

    /// The game clock, in timed modes.
    fn remaining_mut(&mut self) -> Option<&mut f32> {
        match self {
            Self::Rush { remaining } | Self::Tilt { remaining, .. } => Some(remaining),
            Self::Endless => None,
        }
    }

    /// Seconds left on the game clock, in timed modes.
    pub const fn remaining(&self) -> Option<f32> {
        match *self {
            Self::Rush { remaining } | Self::Tilt { remaining, .. } => Some(remaining),
            Self::Endless => None,
        }
    }

    /// Seconds until gravity turns, in the tilt mode.
    pub const fn turn_in(&self) -> Option<f32> {
        match *self {
            Self::Tilt { turn_in, .. } => Some(turn_in),
            _ => None,
        }
    }

    /// Whether gravity is due to turn.
    pub fn turn_due(&self) -> bool {
        self.turn_in() == Some(0.0)
    }

    /// Starts the wait for the next turn.
    pub fn turned(&mut self) {
        if let Self::Tilt { turn_in, .. } = self {
            *turn_in = TURN_SECS;
        }
    }

    /// Adds `time_gems` worth of time (timed modes only).
    pub fn add_time(&mut self, time_gems: u32) {
        if let Some(remaining) = self.remaining_mut() {
            *remaining += time_gems as f32 * TIME_GEM_SECS;
        }
    }

    /// How often new gems carry a time bonus (0: never).
    pub const fn time_gem_odds(&self) -> u32 {
        match self {
            Self::Rush { .. } | Self::Tilt { .. } => TIME_GEM_ODDS,
            Self::Endless => 0,
        }
    }

    /// The name shown to the player.
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Rush { .. } => "Rush",
            Self::Endless => "Endless",
            Self::Tilt { .. } => "Tilt",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rush_runs_out_and_time_gems_extend_it() {
        let mut mode = Mode::rush();
        assert!(!mode.tick(RUSH_SECS - 1.0));
        mode.add_time(2);
        assert!(!mode.tick(1.0 + TIME_GEM_SECS));
        assert!(mode.tick(TIME_GEM_SECS));
        assert!(!Mode::Endless.tick(1e9));
    }

    #[test]
    fn tilt_turns_on_schedule() {
        let mut mode = Mode::tilt();
        mode.tick(TURN_SECS - 0.5);
        assert!(!mode.turn_due());
        mode.tick(1.0);
        assert!(mode.turn_due());
        mode.turned();
        assert_eq!(mode.turn_in(), Some(TURN_SECS));
        assert_eq!(Mode::rush().turn_in(), None);
    }
}
