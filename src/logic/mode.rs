//! The two ways to play.

/// Seconds on the clock at the start of a timed game.
pub const RUSH_SECS: f32 = 60.0;
/// Seconds a time gem adds.
pub const TIME_GEM_SECS: f32 = 5.0;
/// One in this many new gems carries a time bonus in the timed mode.
pub const TIME_GEM_ODDS: u32 = 45;

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
}

impl Mode {
    /// A fresh timed game.
    pub const fn rush() -> Self {
        Self::Rush {
            remaining: RUSH_SECS,
        }
    }

    /// Runs the clock; true once it has run out.
    pub fn tick(&mut self, dt: f32) -> bool {
        match self {
            Self::Rush { remaining } => {
                *remaining = (*remaining - dt).max(0.0);
                *remaining <= 0.0
            }
            Self::Endless => false,
        }
    }

    /// Adds `time_gems` worth of time (timed mode only).
    pub fn add_time(&mut self, time_gems: u32) {
        if let Self::Rush { remaining } = self {
            *remaining += time_gems as f32 * TIME_GEM_SECS;
        }
    }

    /// How often new gems carry a time bonus (0: never).
    pub const fn time_gem_odds(&self) -> u32 {
        match self {
            Self::Rush { .. } => TIME_GEM_ODDS,
            Self::Endless => 0,
        }
    }

    /// Whether this is the same mode as `other`, ignoring its state.
    pub fn same_kind(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }

    /// The name shown to the player.
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Rush { .. } => "Rush",
            Self::Endless => "Endless",
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
}
