//! What a gem does when it is cleared.

/// A gem's special power, earned by bigger matches.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Power {
    /// A plain gem.
    #[default]
    None,
    /// From a match of four: clears the 3x3 cells around it.
    Bomb,
    /// From an L or T shape: clears its whole row and column.
    Cross,
    /// From a match of five: has no colour; swapped with a gem it clears every gem of that
    /// colour, swapped with another wild it clears the board.
    Wild,
}
