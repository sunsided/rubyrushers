//! Where things go on screen: a fixed design space scaled to fit the window.

use crate::logic::{Pos, SIZE};

/// Width of the design space.
pub const DESIGN_W: f32 = 960.0;
/// Height of the design space.
pub const DESIGN_H: f32 = 720.0;
/// Top-left corner of the board in the design space.
pub const BOARD: [f32; 2] = [312.0, 48.0];
/// Edge length of one cell in the design space.
pub const CELL: f32 = 78.0;

/// The design space mapped onto the window: uniformly scaled and centred.
#[derive(Clone, Copy, Debug)]
pub struct Layout {
    /// Window pixels per design unit.
    pub scale: f32,
    /// Window position of the design space's origin.
    pub origin: [f32; 2],
}

impl Layout {
    /// The layout for a window of `w` x `h` pixels.
    pub fn fit(w: f32, h: f32) -> Self {
        let scale = (w / DESIGN_W).min(h / DESIGN_H);
        Self {
            scale,
            origin: [(w - DESIGN_W * scale) / 2.0, (h - DESIGN_H * scale) / 2.0],
        }
    }

    /// The same layout moved by `d` design units (screen shake).
    pub fn shifted(self, d: [f32; 2]) -> Self {
        Self {
            origin: [
                self.origin[0] + d[0] * self.scale,
                self.origin[1] + d[1] * self.scale,
            ],
            ..self
        }
    }

    /// Window position of design point `p`.
    pub fn px(&self, p: [f32; 2]) -> [f32; 2] {
        [
            self.origin[0] + p[0] * self.scale,
            self.origin[1] + p[1] * self.scale,
        ]
    }

    /// Design position of window position `p`.
    pub fn design(&self, p: [f32; 2]) -> [f32; 2] {
        [
            (p[0] - self.origin[0]) / self.scale,
            (p[1] - self.origin[1]) / self.scale,
        ]
    }

    /// A design length in window pixels.
    pub fn len(&self, d: f32) -> f32 {
        d * self.scale
    }

    /// Design position of the centre of (possibly fractional) cell `c`.
    pub fn cell_center(c: [f32; 2]) -> [f32; 2] {
        [
            BOARD[0] + (c[0] + 0.5) * CELL,
            BOARD[1] + (c[1] + 0.5) * CELL,
        ]
    }

    /// The board cell under window position `p`, if any.
    pub fn cell_at(&self, p: [f32; 2]) -> Option<Pos> {
        let d = self.design(p);
        let x = (d[0] - BOARD[0]) / CELL;
        let y = (d[1] - BOARD[1]) / CELL;
        let pos = Pos::new(x.floor() as i32, y.floor() as i32);
        (x >= 0.0 && y >= 0.0 && pos.on_board()).then_some(pos)
    }

    /// The window size of one cell.
    pub fn cell(&self) -> f32 {
        CELL * self.scale
    }
}

/// The board's size in the design space.
pub const BOARD_SIZE: f32 = CELL * SIZE as f32;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cells_round_trip() {
        let l = Layout::fit(1920.0, 1080.0);
        for p in [Pos::new(0, 0), Pos::new(7, 7), Pos::new(3, 5)] {
            let c = Layout::cell_center([p.x as f32, p.y as f32]);
            assert_eq!(l.cell_at(l.px(c)), Some(p));
        }
        assert_eq!(l.cell_at(l.px([BOARD[0] - 1.0, BOARD[1]])), None);
        assert_eq!(
            l.cell_at(l.px([BOARD[0] + BOARD_SIZE + 1.0, BOARD[1]])),
            None
        );
    }
}
