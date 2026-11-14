//! How much the screen moves and sparkles.

/// The effects level. Calm keeps the board readable and steady: for players who prefer
/// reduced motion, and for watching the board with a camera.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Effects {
    /// Everything: shake, sparks, glints, spinning, stretching.
    #[default]
    Full,
    /// No shake, rotation, pulsing or floating; few sparks.
    Calm,
}

impl Effects {
    /// Whether this is the full show.
    #[must_use]
    pub fn full(self) -> bool {
        self == Self::Full
    }

    /// The other level.
    #[must_use]
    pub fn toggled(self) -> Self {
        match self {
            Self::Full => Self::Calm,
            Self::Calm => Self::Full,
        }
    }

    /// The name shown to the player.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Full => "Full",
            Self::Calm => "Calm",
        }
    }
}
