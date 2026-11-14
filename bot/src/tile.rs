//! What the bot thinks is in one cell.

/// One cell as seen: a gem of some colour class, a wild gem, or unreadable.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Tile {
    /// A gem of colour class `0..7` (red, orange, yellow, green, blue, purple, white).
    Gem(u8),
    /// A rainbow gem that matches nothing but can swap with anything.
    Wild,
    /// Moving, covered or otherwise unreadable.
    Unknown,
}

impl Tile {
    /// Whether two tiles line up in a match.
    pub fn matches(self, other: Self) -> bool {
        matches!((self, other), (Self::Gem(a), Self::Gem(b)) if a == b)
    }
}
