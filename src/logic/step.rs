//! What happened during a move, in order, for the view to play back.

use super::gem::Gem;
use super::pos::Pos;
use super::power::Power;

/// One step of a resolved move.
#[derive(Clone, Debug, PartialEq)]
pub enum Step {
    /// Two gems traded places, or tried to (`ok == false`: they swap back).
    Swap {
        /// The first cell and the id of the gem that started there.
        a: (Pos, u32),
        /// The second cell and the id of the gem that started there.
        b: (Pos, u32),
        /// Whether the swap stands.
        ok: bool,
    },
    /// Gems were cleared.
    Clear(Clear),
    /// Gems fell into the gaps and new ones dropped in from above.
    Settle {
        /// Moved gem ids and their new cells.
        falls: Vec<(u32, Pos)>,
        /// New gems, their cells and the (negative) row they start falling from.
        spawns: Vec<(Gem, Pos, i32)>,
    },
    /// No move was left: every gem moved to a new cell.
    Reshuffle {
        /// Every gem and its cell.
        gems: Vec<(Gem, Pos)>,
    },
}

/// One cascade step's clearing.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Clear {
    /// 1 for the player's match, 2 for the first cascade, and so on.
    pub cascade: u32,
    /// The cleared gems and where they were.
    pub cleared: Vec<(Gem, Pos)>,
    /// Special gems that went off: the power, where, and the cells it took out.
    pub blasts: Vec<(Power, Pos, Vec<Pos>)>,
    /// Special gems earned by this step's matches, already on the board.
    pub created: Vec<(Gem, Pos)>,
    /// Points scored.
    pub points: u64,
    /// Time gems among the cleared.
    pub time_gems: u32,
}
