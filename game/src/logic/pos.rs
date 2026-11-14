//! A cell on the board.

use super::board::SIZE;

/// A cell: column `x` from the left, row `y` from the top, both `0..SIZE` on the board.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Pos {
    /// Column, from the left.
    pub x: i32,
    /// Row, from the top.
    pub y: i32,
}

impl Pos {
    /// The cell at column `x`, row `y`.
    #[must_use]
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Whether the cell lies on the board.
    #[must_use]
    pub const fn on_board(self) -> bool {
        self.x >= 0 && self.x < SIZE && self.y >= 0 && self.y < SIZE
    }

    /// Whether `other` shares an edge with this cell.
    #[must_use]
    pub const fn adjacent(self, other: Self) -> bool {
        (self.x - other.x).abs() + (self.y - other.y).abs() == 1
    }

    /// Every cell of the board, row by row.
    pub fn all() -> impl Iterator<Item = Self> {
        (0..SIZE).flat_map(|y| (0..SIZE).map(move |x| Self::new(x, y)))
    }
}
