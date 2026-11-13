//! The rules of the game, free of the toolkit: board, matching, cascades, scoring, modes.
//!
//! A move resolves instantly into a list of [`Step`]s; the view plays them back as
//! animations. Everything here is deterministic for a given seed and headless-testable.

pub mod board;
pub mod gem;
pub mod matcher;
pub mod mode;
pub mod moves;
pub mod pos;
pub mod power;
pub mod resolve;
pub mod rng;
pub mod score;
pub mod session;
pub mod step;

pub use board::{Board, SIZE};
pub use gem::{COLORS, Gem};
pub use mode::Mode;
pub use pos::Pos;
pub use power::Power;
pub use session::Session;
pub use step::{Clear, Step};
