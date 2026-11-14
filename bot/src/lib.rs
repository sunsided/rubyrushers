//! The RubyRushers autoplayer: sees the game only through screenshots and plays it
//! through mouse clicks, like a person at the screen would.
//!
//! It never links the game. Everything it knows comes from pixels: [`grid`] finds the
//! board, [`read`] tells the gems apart, [`solver`] picks a swap with its own idea of
//! the rules, and [`browser`] takes screenshots and clicks.

pub mod board;
pub mod browser;
pub mod cdp;
pub mod grid;
pub mod image;
pub mod read;
pub mod serve;
pub mod sim;
pub mod solver;
pub mod tile;

pub use board::{Board, SIZE};
pub use grid::Grid;
pub use image::Image;
pub use tile::Tile;
