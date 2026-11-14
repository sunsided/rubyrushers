//! The board as the bot reads it.

use crate::tile::Tile;

/// Cells per side.
pub const SIZE: usize = 8;

/// Tiles by row (`[y][x]`, row 0 at the top).
pub type Board = [[Tile; SIZE]; SIZE];

/// Whether every cell was read.
pub fn complete(board: &Board) -> bool {
    board.iter().flatten().all(|t| *t != Tile::Unknown)
}
