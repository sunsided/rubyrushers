//! Finding the board in a screenshot.
//!
//! The board is wherever 64 similar, roughly square, colourful blobs sit on a regular
//! 8x8 lattice. Nothing about the game's layout is assumed beyond that.

use anyhow::{Result, bail};

use crate::board::SIZE;
use crate::image::{Image, hsv};

/// Where the cells are on screen.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Grid {
    /// Pixel position of the top-left corner of cell (0, 0).
    pub origin: [f32; 2],
    /// Pixels from one cell to the next.
    pub pitch: f32,
}

impl Grid {
    /// The pixel centre of cell (`x`, `y`).
    pub fn center(&self, x: usize, y: usize) -> [f32; 2] {
        [
            self.origin[0] + (x as f32 + 0.5) * self.pitch,
            self.origin[1] + (y as f32 + 0.5) * self.pitch,
        ]
    }

    /// Finds the board in `img`. Fails while the board is hidden or too much of it moves.
    pub fn locate(img: &Image) -> Result<Self> {
        let blobs = blobs(img);
        // Gem-sized blobs: roughly square, the size most blobs share.
        let mut squares: Vec<&Blob> = blobs
            .iter()
            .filter(|b| {
                let (w, h) = b.size();
                w >= 12.0 && h >= 12.0 && (0.6..=1.6).contains(&(w / h))
            })
            .collect();
        if squares.len() < SIZE * SIZE / 2 {
            bail!("only {} gem-like blobs", squares.len());
        }
        let mut sizes: Vec<f32> = squares.iter().map(|b| b.size().0.max(b.size().1)).collect();
        sizes.sort_by(f32::total_cmp);
        let typical = sizes[sizes.len() / 2];
        squares.retain(|b| {
            let s = b.size().0.max(b.size().1);
            (typical * 0.7..=typical * 1.3).contains(&s)
        });

        // Columns first; only blobs in those columns count for the rows, so text and
        // effects beside the board cannot pass for a row.
        let xs = lattice(squares.iter().map(|b| b.center()[0]), typical)?;
        let in_column = |b: &&&Blob| {
            let k = (b.center()[0] - xs.0) / xs.1;
            k > -0.5 && k < SIZE as f32 - 0.5 && (k - k.round()).abs() < 0.2
        };
        let ys = lattice(
            squares.iter().filter(in_column).map(|b| b.center()[1]),
            typical,
        )?;
        let pitch = (xs.1 + ys.1) / 2.0;
        Ok(Self {
            origin: [xs.0 - pitch / 2.0, ys.0 - pitch / 2.0],
            pitch,
        })
    }
}

/// From blob centres along one axis: the first of [`SIZE`] evenly spaced lines and the
/// spacing. Centres closer than `typical / 2` belong to one line; lines with fewer than
/// two blobs are noise.
fn lattice(centers: impl Iterator<Item = f32>, typical: f32) -> Result<(f32, f32)> {
    let mut cs: Vec<f32> = centers.collect();
    cs.sort_by(f32::total_cmp);
    let mut lines: Vec<(f32, usize)> = Vec::new(); // (sum, count)
    let mut last = f32::NEG_INFINITY;
    for c in cs {
        if c - last > typical / 2.0 {
            lines.push((0.0, 0));
        }
        let l = lines.last_mut().expect("pushed above");
        l.0 += c;
        l.1 += 1;
        last = c;
    }
    let lines: Vec<(f32, usize)> = lines
        .iter()
        .filter(|(_, n)| *n >= 2)
        .map(|(s, n)| (s / *n as f32, *n))
        .collect();
    // Of all runs of SIZE evenly spaced lines, the one most blobs sit on.
    let mut best: Option<(usize, f32, f32)> = None;
    for run in lines.windows(SIZE) {
        let first = run[0].0;
        let pitch = (run[SIZE - 1].0 - first) / (SIZE - 1) as f32;
        let even = run
            .iter()
            .enumerate()
            .all(|(i, (m, _))| (m - (first + i as f32 * pitch)).abs() < pitch * 0.15);
        let support: usize = run.iter().map(|(_, n)| n).sum();
        if even && pitch > typical * 0.8 && best.is_none_or(|b| support > b.0) {
            best = Some((support, first, pitch));
        }
    }
    best.map(|(_, first, pitch)| (first, pitch))
        .ok_or_else(|| anyhow::anyhow!("no {SIZE} evenly spaced lines among {}", lines.len()))
}

/// Whether a pixel looks like part of a gem: colourful, or bright and white.
fn gemlike(c: [u8; 3]) -> bool {
    let (_, s, v) = hsv(c);
    (s > 0.35 && v > 0.3) || (s < 0.2 && v > 0.75)
}

/// A connected patch of gem-like pixels.
#[derive(Clone, Copy, Debug)]
struct Blob {
    /// Bounding box: left, top, right, bottom (inclusive).
    bbox: [usize; 4],
}

impl Blob {
    /// Width and height.
    fn size(&self) -> (f32, f32) {
        (
            (self.bbox[2] - self.bbox[0] + 1) as f32,
            (self.bbox[3] - self.bbox[1] + 1) as f32,
        )
    }

    /// Centre of the bounding box.
    fn center(&self) -> [f32; 2] {
        [
            (self.bbox[0] + self.bbox[2]) as f32 / 2.0 + 0.5,
            (self.bbox[1] + self.bbox[3]) as f32 / 2.0 + 0.5,
        ]
    }
}

/// All 4-connected patches of gem-like pixels.
fn blobs(img: &Image) -> Vec<Blob> {
    let (w, h) = (img.width, img.height);
    let mask: Vec<bool> = (0..w * h).map(|i| gemlike(img.at(i % w, i / w))).collect();
    let mut seen = vec![false; w * h];
    let mut out = Vec::new();
    let mut stack = Vec::new();
    for start in 0..w * h {
        if !mask[start] || seen[start] {
            continue;
        }
        seen[start] = true;
        stack.push(start);
        let mut bbox = [start % w, start / w, start % w, start / w];
        while let Some(i) = stack.pop() {
            let (x, y) = (i % w, i / w);
            bbox = [
                bbox[0].min(x),
                bbox[1].min(y),
                bbox[2].max(x),
                bbox[3].max(y),
            ];
            let mut visit = |j: usize| {
                if mask[j] && !seen[j] {
                    seen[j] = true;
                    stack.push(j);
                }
            };
            if x > 0 {
                visit(i - 1);
            }
            if x + 1 < w {
                visit(i + 1);
            }
            if y > 0 {
                visit(i - w);
            }
            if y + 1 < h {
                visit(i + w);
            }
        }
        out.push(Blob { bbox });
    }
    out
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Colours a synthetic board uses, one per class.
    pub const PALETTE: [[u8; 3]; 7] = [
        [230, 25, 56],
        [255, 133, 20],
        [255, 217, 38],
        [31, 199, 82],
        [46, 117, 255],
        [168, 66, 245],
        [235, 240, 255],
    ];

    /// A dark screen with an 8x8 board of filled squares at `origin`/`pitch` coloured by
    /// `class(x, y)`, plus some bright text-like specks on the left.
    pub fn synthetic(
        origin: [usize; 2],
        pitch: usize,
        class: impl Fn(usize, usize) -> usize,
    ) -> Image {
        let mut img = Image::filled(1000, 760, [20, 14, 40]);
        for gy in 0..SIZE {
            for gx in 0..SIZE {
                let c = PALETTE[class(gx, gy)];
                let inset = pitch / 8;
                for y in inset..pitch - inset {
                    for x in inset..pitch - inset {
                        img.set(origin[0] + gx * pitch + x, origin[1] + gy * pitch + y, c);
                    }
                }
            }
        }
        // "Text": thin bright strokes.
        for i in 0..20 {
            for y in 40..70 {
                img.set(30 + i * 9, y, [255, 255, 255]);
            }
        }
        img
    }

    #[test]
    fn finds_a_synthetic_board() {
        let img = synthetic([300, 50], 80, |x, y| (x * 3 + y) % 7);
        let g = Grid::locate(&img).unwrap();
        assert!((g.pitch - 80.0).abs() < 1.0, "{g:?}");
        assert!(
            (g.origin[0] - 300.0).abs() < 2.0 && (g.origin[1] - 50.0).abs() < 2.0,
            "{g:?}"
        );
        let c = g.center(7, 7);
        assert!((c[0] - (300.0 + 7.5 * 80.0)).abs() < 2.0);
    }

    #[test]
    fn an_empty_screen_has_no_board() {
        assert!(Grid::locate(&Image::filled(400, 300, [10, 10, 30])).is_err());
    }
}
