//! RubyRushers: a match-three gem game on game-toolkit.
//!
//! [`logic`] holds the rules and knows nothing of the toolkit; [`app`] is the toolkit glue,
//! [`render`] draws, [`sound`] makes the noises.

pub mod app;
pub mod logic;
pub mod render;
pub mod scores;
pub mod sound;
