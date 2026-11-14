//! Which way gems fall.

use super::board::SIZE;
use super::pos::Pos;

/// The direction gems fall in. Each column (falling up or down) or row (falling left or
/// right) is a lane; depth counts cells from the edge gems fall towards.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Gravity {
    /// Towards the bottom edge; new gems come in at the top.
    #[default]
    Down,
    /// Towards the left edge.
    Left,
    /// Towards the top edge.
    Up,
    /// Towards the right edge.
    Right,
}

impl Gravity {
    /// The direction of the fall, one cell long.
    #[must_use]
    pub const fn dir(self) -> [f32; 2] {
        match self {
            Self::Down => [0.0, 1.0],
            Self::Left => [-1.0, 0.0],
            Self::Up => [0.0, -1.0],
            Self::Right => [1.0, 0.0],
        }
    }

    /// The cell in `lane` at `depth` cells from the edge gems fall towards.
    #[must_use]
    pub const fn cell(self, lane: i32, depth: i32) -> Pos {
        match self {
            Self::Down => Pos::new(lane, SIZE - 1 - depth),
            Self::Up => Pos::new(lane, depth),
            Self::Right => Pos::new(SIZE - 1 - depth, lane),
            Self::Left => Pos::new(depth, lane),
        }
    }

    /// The lane of cell `p`.
    #[must_use]
    pub const fn lane(self, p: Pos) -> i32 {
        match self {
            Self::Down | Self::Up => p.x,
            Self::Left | Self::Right => p.y,
        }
    }

    /// The next direction, a quarter turn clockwise (down, left, up, right).
    #[must_use]
    pub const fn turned(self) -> Self {
        match self {
            Self::Down => Self::Left,
            Self::Left => Self::Up,
            Self::Up => Self::Right,
            Self::Right => Self::Down,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lanes_and_depths_cover_the_board_once() {
        for g in [Gravity::Down, Gravity::Left, Gravity::Up, Gravity::Right] {
            let mut cells: Vec<Pos> = (0..SIZE)
                .flat_map(|l| (0..SIZE).map(move |d| g.cell(l, d)))
                .collect();
            assert!(cells.iter().all(|p| p.on_board()));
            cells.sort_unstable();
            cells.dedup();
            assert_eq!(cells.len(), Pos::all().count());
            // Depth 0 lies on the edge the fall points to.
            let edge = g.cell(3, 0);
            #[expect(
                clippy::cast_possible_truncation,
                reason = "dir() is a unit step: each component is -1, 0 or 1"
            )]
            let beyond = Pos::new(edge.x + g.dir()[0] as i32, edge.y + g.dir()[1] as i32);
            assert!(!beyond.on_board(), "{g:?}");
            assert_eq!(g.lane(g.cell(5, 2)), 5);
        }

        assert_eq!(Gravity::Right.turned(), Gravity::Down);
    }
}
