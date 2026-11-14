//! The rules of the game, free of the toolkit: board, matching, cascades, scoring, modes.
//!
//! The board runs in real time: [`Session::update`] moves the gems, matches whatever
//! rests, and reports [`Event`]s. Deterministic for a seed and a sequence of updates and
//! moves, and headless-testable.

pub mod blast;
pub mod board;
pub mod event;
pub mod field;
pub mod gem;
pub mod gravity;
pub mod matcher;
pub mod mode;
pub mod motion;
pub mod moves;
pub mod pos;
pub mod power;
pub mod rng;
pub mod score;
pub mod session;

pub use board::{Board, SIZE};
pub use event::{Clear, Event};
pub use field::Piece;
pub use gem::{COLORS, Gem};
pub use gravity::Gravity;
pub use mode::Mode;
pub use pos::Pos;
pub use power::Power;
pub use session::Session;
