//! `RubyRushers`: a match-three gem game on game-toolkit.
//!
//! [`logic`] holds the rules and knows nothing of the toolkit; [`app`] is the toolkit glue,
//! [`render`] draws, [`sound`] makes the noises, and [`host`] hears from the page the
//! browser build runs in.

pub mod app;
pub mod host;
pub mod logic;
pub mod render;
pub mod scores;
pub mod sound;
