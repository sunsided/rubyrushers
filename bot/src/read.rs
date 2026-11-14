//! Reading the gems off the board.

use crate::board::{Board, SIZE};
use crate::grid::Grid;
use crate::image::{Image, hsv};
use crate::tile::Tile;

/// Hue (degrees) of each colour class, white aside.
const HUES: [f32; 6] = [352.0, 29.0, 49.0, 138.0, 220.0, 274.0];
/// The class of white gems.
const WHITE: u8 = 6;

/// Reads every cell of the board.
pub fn board(img: &Image, grid: &Grid) -> Board {
    let mut b = [[Tile::Unknown; SIZE]; SIZE];
    for (y, row) in b.iter_mut().enumerate() {
        for (x, t) in row.iter_mut().enumerate() {
            *t = cell(img, grid.center(x, y), grid.pitch);
        }
    }
    b
}

/// Shortest distance between two hues, in degrees.
fn hue_distance(a: f32, b: f32) -> f32 {
    let d = (a - b).rem_euclid(360.0);
    d.min(360.0 - d)
}

/// Reads one cell from the patch around its centre (inside any rings drawn around the
/// gem): the dominant hue of the colourful pixels, white when there are hardly any, wild
/// when the hues are all over the place.
pub fn cell(img: &Image, center: [f32; 2], pitch: f32) -> Tile {
    let r = (pitch * 0.2) as isize;
    let (cx, cy) = (center[0] as isize, center[1] as isize);
    let (mut n, mut colourful, mut bright) = (0usize, 0usize, 0usize);
    let (mut sx, mut sy) = (0.0f32, 0.0f32);
    for y in cy - r..=cy + r {
        for x in cx - r..=cx + r {
            if x < 0 || y < 0 || x as usize >= img.width || y as usize >= img.height {
                continue;
            }
            n += 1;
            let (h, s, v) = hsv(img.at(x as usize, y as usize));
            if s > 0.35 && v > 0.3 {
                colourful += 1;
                let a = h.to_radians();
                sx += a.cos();
                sy += a.sin();
            } else if s < 0.2 && v > 0.6 {
                bright += 1;
            }
        }
    }
    if n == 0 {
        return Tile::Unknown;
    }
    if colourful * 4 < n {
        return if bright * 2 > n {
            Tile::Gem(WHITE)
        } else {
            Tile::Unknown
        };
    }
    // How well the hues agree: 1 all the same, 0 spread around the circle.
    let agreement = (sx * sx + sy * sy).sqrt() / colourful as f32;
    if agreement < 0.6 {
        return Tile::Wild;
    }
    let hue = sy.atan2(sx).to_degrees().rem_euclid(360.0);
    let (class, d) = HUES
        .iter()
        .enumerate()
        .map(|(i, h)| (i, hue_distance(hue, *h)))
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .expect("six hues");
    if d > 25.0 {
        Tile::Unknown
    } else {
        Tile::Gem(class as u8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::tests::synthetic;

    #[test]
    fn reads_a_synthetic_board() {
        let class = |x: usize, y: usize| (x * 3 + y * 5) % 7;
        let img = synthetic([300, 50], 80, class);
        let grid = Grid::locate(&img).unwrap();
        let b = board(&img, &grid);
        for (y, row) in b.iter().enumerate() {
            for (x, tile) in row.iter().enumerate() {
                assert_eq!(*tile, Tile::Gem(class(x, y) as u8), "({x}, {y})");
            }
        }
    }

    #[test]
    fn a_rainbow_reads_as_wild() {
        let mut img = Image::filled(100, 100, [0, 0, 0]);
        for y in 0..100 {
            for x in 0..100 {
                let a = ((y as f32 - 50.0).atan2(x as f32 - 50.0).to_degrees() + 360.0) % 360.0;
                let sector = (a / 60.0) as usize;
                let c = [
                    [255, 0, 0],
                    [255, 255, 0],
                    [0, 255, 0],
                    [0, 255, 255],
                    [0, 0, 255],
                    [255, 0, 255],
                ][sector % 6];
                img.set(x, y, c);
            }
        }
        assert_eq!(cell(&img, [50.0, 50.0], 100.0), Tile::Wild);
    }

    #[test]
    fn darkness_is_unknown() {
        let img = Image::filled(100, 100, [10, 10, 20]);
        assert_eq!(cell(&img, [50.0, 50.0], 100.0), Tile::Unknown);
    }
}
