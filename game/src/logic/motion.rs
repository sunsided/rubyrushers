//! How a gem is moving relative to its cell.

/// Seconds a swap takes.
pub const SWAP_SECS: f32 = 0.15;
/// Seconds a swap that lines nothing up takes to go and come back.
pub const BOUNCE_SECS: f32 = 0.26;
/// Falling acceleration in cells per second squared.
pub const GRAVITY: f32 = 70.0;

/// A swap animation, with the gem already in its new cell.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Swap {
    /// From the gem's cell towards where it came from (move) or where it tries to go
    /// (bounce), in cells.
    pub dir: [f32; 2],
    /// Seconds since the start.
    pub t: f32,
    /// A failed swap: go halfway and come back.
    pub bounce: bool,
}

impl Swap {
    /// The current offset from the cell, in cells.
    pub fn offset(&self) -> [f32; 2] {
        let f = if self.bounce {
            let k = (self.t / BOUNCE_SECS).min(1.0);
            0.5 * (1.0 - (2.0 * k - 1.0).abs())
        } else {
            let k = (self.t / SWAP_SECS).min(1.0);
            // Ease in and out.
            let e = if k < 0.5 {
                4.0 * k * k * k
            } else {
                1.0 - (-2.0 * k + 2.0).powi(3) / 2.0
            };
            1.0 - e
        };
        [self.dir[0] * f, self.dir[1] * f]
    }

    /// Whether the animation has ended.
    pub fn done(&self) -> bool {
        self.t >= if self.bounce { BOUNCE_SECS } else { SWAP_SECS }
    }
}

/// A gem's motion and what it remembers about how it got where it is.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Motion {
    /// A running swap.
    pub swap: Option<Swap>,
    /// Cells it still has to fall to reach its cell, negated (0 or negative).
    pub fall: f32,
    /// Falling speed in cells per second.
    pub vy: f32,
    /// The cascade depth of the clear that made it move (0: it has not moved since).
    pub chain: u32,
    /// The player swapped it here (a reward gem appears in its cell).
    pub moved: bool,
}

impl Motion {
    /// Whether the gem rests in its cell: it can match and be swapped.
    pub fn settled(&self) -> bool {
        self.swap.is_none() && self.fall == 0.0
    }

    /// Offset of the drawn gem from its cell, in cells, for gems falling along `dir`.
    pub fn offset(&self, dir: [f32; 2]) -> [f32; 2] {
        let s = self.swap.map_or([0.0; 2], |s| s.offset());
        [s[0] + dir[0] * self.fall, s[1] + dir[1] * self.fall]
    }
}
