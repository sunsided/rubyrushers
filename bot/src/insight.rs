//! What the bot saw and thought in one round, for the viewer (`--watch`): the pixels it
//! captured, the tiles it read and trusted, the moves it weighed and the board it expects.

use std::sync::Mutex;
use std::sync::atomic::AtomicBool;

use crate::board::{Board, SIZE};
use crate::grid::Grid;
use crate::image::Image;
use crate::solver::{Swap, Value};

/// The latest round, waiting for the viewer to pick it up.
pub static LATEST: Mutex<Option<Insight>> = Mutex::new(None);
/// Set when the viewer closes: the bot stops (and closes its browser).
pub static STOP: AtomicBool = AtomicBool::new(false);
/// Set when the bot is done playing.
pub static FINISHED: AtomicBool = AtomicBool::new(false);

/// The move the bot made and what it expects of it.
#[derive(Clone, Debug)]
pub struct Plan {
    /// The trusted board it was chosen on.
    pub before: Board,
    /// The best moves on that board, best first (a few): this one and its rivals.
    pub candidates: Vec<(Swap, Value)>,
    /// The swap.
    pub swap: Swap,
    /// Its value.
    pub value: Value,
    /// The board once it and its cascades settled; unknown where new gems come in.
    pub after: Board,
    /// The best move on that board: what this one sets up.
    pub next: Option<(Swap, Value)>,
}

/// One round of the bot's loop.
#[derive(Clone, Debug)]
pub struct Insight {
    /// The captured board area.
    pub crop: Image,
    /// Where the cells are in `crop`.
    pub view: Grid,
    /// Every cell as read from `crop`.
    pub read: Board,
    /// The cells the bot relies on: read alike twice and not busy with its own moves.
    pub trusted: Board,
    /// Cells its own recent moves are still expected to change.
    pub busy: [[bool; SIZE]; SIZE],
    /// The move it made this round, if any (the viewer keeps showing the last one).
    pub plan: Option<Plan>,
    /// Moves made so far.
    pub moves: u32,
    /// Seconds since play began.
    pub elapsed: f32,
    /// Average milliseconds per round: capture, read, solve, click.
    pub timings: [f32; 4],
    /// What the bot is doing, in a few words.
    pub status: String,
}
