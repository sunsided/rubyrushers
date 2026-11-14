//! A gem on the board.

use super::power::Power;

/// Number of gem colours.
pub const COLORS: u8 = 7;

/// A gem. The `id` follows it as it moves, so the view can animate it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Gem {
    /// Unique per board, for tracking the gem across moves.
    pub id: u32,
    /// `0..COLORS`; meaningless for a [`Power::Wild`] gem.
    pub color: u8,
    /// What it does when cleared.
    pub power: Power,
    /// Clearing it adds time in the timed mode.
    pub time_bonus: bool,
}

impl Gem {
    /// Whether the gem lines up with `other` in a match: same colour, neither wild.
    pub fn matches(&self, other: &Self) -> bool {
        self.power != Power::Wild && other.power != Power::Wild && self.color == other.color
    }
}
