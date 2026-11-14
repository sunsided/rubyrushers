//! What happened on the board, for sound and effects.

use super::gem::Gem;
use super::gravity::Gravity;
use super::pos::Pos;
use super::power::Power;

/// Something that happened during a tick.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// The player swapped two gems.
    Swapped,
    /// The player tried a swap that lines nothing up; the gems bounce back.
    Rejected,
    /// Gems were cleared.
    Cleared(Clear),
    /// Falling gems landed.
    Landed,
    /// No move was left: the gems were mixed and drop in anew.
    Reshuffled,
    /// Gravity turned to this direction.
    Tilted(Gravity),
    /// The game ended.
    Over,
}

/// Gems cleared together in one tick.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Clear {
    /// 1 for a match the player made, 2 for one their move caused, and so on.
    pub cascade: u32,
    /// The cleared gems and where they were drawn (cells, fractional).
    pub cleared: Vec<(Gem, [f32; 2])>,
    /// Special gems that went off: the power, where, and the cells it took out.
    pub blasts: Vec<(Power, Pos, Vec<Pos>)>,
    /// Special gems earned by the matches, already on the board.
    pub created: Vec<(Gem, Pos)>,
    /// Points scored.
    pub points: u64,
    /// Time gems among the cleared.
    pub time_gems: u32,
}
