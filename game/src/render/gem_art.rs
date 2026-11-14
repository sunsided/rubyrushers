//! Gem pictures, rasterised once at start-up into one atlas texture.
//!
//! Every colour has its own silhouette, so the gems tell apart by shape as well as by
//! colour. A cut gem is a flat table in the middle and a ring of bevel facets around it,
//! each facet lit by its direction to a light from the upper left.

use std::f32::consts::{PI, TAU};

use crate::logic::COLORS;

/// Edge length of one atlas tile in pixels.
pub const TILE: u32 = 128;
/// Tiles in the atlas: one per colour, then the wild gem.
pub const TILES: u32 = COLORS as u32 + 1;
/// The atlas tile of the wild gem.
pub const WILD_TILE: u32 = COLORS as u32;

/// Base colour of each gem colour, linear-ish RGB in `0.0..=1.0`.
pub const BASE: [[f32; 3]; COLORS as usize] = [
    [0.90, 0.10, 0.22], // ruby: round
    [1.00, 0.52, 0.08], // amber: triangle
    [1.00, 0.85, 0.15], // topaz: star
    [0.12, 0.78, 0.32], // emerald: square
    [0.18, 0.46, 1.00], // sapphire: rhombus
    [0.66, 0.26, 0.96], // amethyst: hexagon
    [0.90, 0.93, 1.00], // diamond: octagon
];

/// The atlas: `TILES` tiles side by side, RGBA8, `TILE * TILES` × `TILE` pixels.
pub fn atlas() -> Vec<u8> {
    let w = TILE * TILES;
    let mut rgba = vec![0u8; (w * TILE * 4) as usize];

    for tile in 0..TILES {
        for y in 0..TILE {
            for x in 0..TILE {
                let i = ((y * w + tile * TILE + x) * 4) as usize;
                rgba[i..i + 4].copy_from_slice(&texel(tile, x, y));
            }
        }
    }

    rgba
}

/// Pixel (`x`, `y`) of tile `tile`, as RGBA8.
fn texel(tile: u32, x: u32, y: u32) -> [u8; 4] {
    let c = if tile == WILD_TILE {
        wild_pixel(x, y)
    } else {
        gem_pixel(tile as usize, x, y)
    };

    c.map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8)
}

/// The outline of gem colour `color`: a polygon around the origin, star-shaped from it,
/// in units of the gem radius (y down).
fn outline(color: usize) -> Vec<[f32; 2]> {
    let regular = |n: usize, radius: f32, start_deg: f32| -> Vec<[f32; 2]> {
        (0..n)
            .map(|i| {
                let a = start_deg.to_radians() + TAU * i as f32 / n as f32;
                [radius * a.cos(), radius * a.sin()]
            })
            .collect()
    };

    match color {
        0 => regular(16, 0.88, 0.0),
        1 => regular(3, 1.08, -90.0)
            .into_iter()
            .map(|[x, y]| [x, y + 0.14])
            .collect(),
        2 => (0..10)
            .map(|i| {
                let a = -PI / 2.0 + PI * i as f32 / 5.0;
                let r = if i % 2 == 0 { 1.06 } else { 0.52 };
                [r * a.cos(), r * a.sin() + 0.05]
            })
            .collect(),
        3 => regular(4, 1.08, 45.0),
        4 => vec![[0.0, -1.08], [0.74, 0.0], [0.0, 1.08], [-0.74, 0.0]],
        5 => regular(6, 1.0, 0.0),
        _ => regular(8, 1.0, 22.5),
    }
}

/// Distance from the origin to the outline along the direction `d` (unit length), and the
/// index of the edge it crosses.
fn reach(poly: &[[f32; 2]], d: [f32; 2]) -> (f32, usize) {
    let mut best = (f32::MAX, 0);

    for i in 0..poly.len() {
        let a = poly[i];
        let b = poly[(i + 1) % poly.len()];
        let e = [b[0] - a[0], b[1] - a[1]];
        let den = d[0] * e[1] - d[1] * e[0];

        if den.abs() < 1e-9 {
            continue;
        }

        // Solve t*d = a + s*e.
        let t = (a[0] * e[1] - a[1] * e[0]) / den;
        let s = (a[0] * d[1] - a[1] * d[0]) / den;

        if t > 0.0 && (-1e-4..=1.0 + 1e-4).contains(&s) && t < best.0 {
            best = (t, i);
        }
    }

    best
}

/// Gem radius in pixels within its tile.
const RADIUS: f32 = TILE as f32 * 0.44;
/// Direction the light comes from (upper left), unit length.
const LIGHT: [f32; 2] = [-0.55, -0.835];

/// One pixel of the gem of colour `color`, RGBA.
fn gem_pixel(color: usize, x: u32, y: u32) -> [f32; 4] {
    let poly = outline(color);
    let half = TILE as f32 / 2.0;
    let p = [
        (x as f32 + 0.5 - half) / RADIUS,
        (y as f32 + 0.5 - half) / RADIUS,
    ];
    let r = (p[0] * p[0] + p[1] * p[1]).sqrt();

    if r < 1e-6 {
        return shade(color, 1.0, 0.0, p);
    }

    let d = [p[0] / r, p[1] / r];
    let (edge_r, edge) = reach(&poly, d);
    // Anti-aliased coverage over about one pixel at the outline.
    let coverage = ((edge_r - r) * RADIUS + 0.5).clamp(0.0, 1.0);

    if coverage <= 0.0 {
        return [0.0; 4];
    }

    // The edge's outward normal says which way the facet faces.
    let a = poly[edge];
    let b = poly[(edge + 1) % poly.len()];
    let mut n = [b[1] - a[1], a[0] - b[0]];
    let len = (n[0] * n[0] + n[1] * n[1]).sqrt();
    n = [n[0] / len, n[1] / len];

    if n[0] * a[0] + n[1] * a[1] < 0.0 {
        n = [-n[0], -n[1]];
    }

    let facing = n[0] * LIGHT[0] + n[1] * LIGHT[1];
    let depth = r / edge_r;
    let mut c = shade(color, depth, facing, p);

    // A dark rim along the outline.
    if (edge_r - r) * RADIUS < 2.5 {
        for v in &mut c[..3] {
            *v *= 0.55;
        }
    }

    c[3] = coverage;

    c
}

/// The colour at relative depth `depth` (0 centre, 1 outline) of a gem whose facet there
/// faces the light by `facing` (-1..1); `p` is the position for the highlight.
fn shade(color: usize, depth: f32, facing: f32, p: [f32; 2]) -> [f32; 4] {
    let base = BASE[color];

    let light = if depth < 0.52 {
        // The table: flat, brighter towards the top.
        1.0 - p[1] * 0.25
    } else if depth < 0.8 {
        0.85 + 0.3 * facing
    } else {
        0.6 + 0.5 * facing
    };
    // A soft specular spot on the upper left.
    let dx = p[0] + 0.32;
    let dy = p[1] + 0.38;
    let spec = (-(dx * dx + dy * dy) / 0.035).exp() * 0.75;
    let mut c = [0.0, 0.0, 0.0, 1.0];

    for k in 0..3 {
        c[k] = base[k] * light + spec + (1.0 - base[k]) * 0.12 * facing.max(0.0);
    }

    c
}

/// One pixel of the wild gem: a rainbow swirl in a dark-rimmed orb.
fn wild_pixel(x: u32, y: u32) -> [f32; 4] {
    let half = TILE as f32 / 2.0;
    let p = [
        (x as f32 + 0.5 - half) / RADIUS,
        (y as f32 + 0.5 - half) / RADIUS,
    ];
    let r = (p[0] * p[0] + p[1] * p[1]).sqrt();
    let coverage = ((1.0 - r) * RADIUS + 0.5).clamp(0.0, 1.0);

    if coverage <= 0.0 {
        return [0.0; 4];
    }

    let hue = (p[1].atan2(p[0]) / TAU + r * 0.7).rem_euclid(1.0);
    let [hr, hg, hb] = hue_rgb(hue);
    let core = (1.0 - r * 1.6).max(0.0);
    let rim = if r > 0.9 { 0.5 } else { 1.0 };
    let dx = p[0] + 0.3;
    let dy = p[1] + 0.35;
    let spec = (-(dx * dx + dy * dy) / 0.03).exp() * 0.8;
    let mix = |h: f32| ((h * (0.75 + 0.25 * (1.0 - r)) + core) * rim + spec).min(1.0);

    [mix(hr), mix(hg), mix(hb), coverage]
}

/// A fully saturated colour for `hue` in `0.0..1.0`.
fn hue_rgb(hue: f32) -> [f32; 3] {
    let h = hue * 6.0;
    let f = |n: f32| (((n + h) % 6.0 - 3.0).abs() - 1.0).clamp(0.0, 1.0);
    [f(0.0), f(4.0), f(2.0)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atlas_has_opaque_centres_and_clear_corners() {
        let rgba = atlas();
        let w = TILE * TILES;
        let alpha = |x: u32, y: u32| rgba[((y * w + x) * 4 + 3) as usize];

        for tile in 0..TILES {
            assert_eq!(alpha(tile * TILE + TILE / 2, TILE / 2), 255, "tile {tile}");
            assert_eq!(alpha(tile * TILE + 1, 1), 0, "tile {tile}");
        }
    }

    #[test]
    fn silhouettes_differ() {
        // Coverage masks of every pair of colours differ in a good share of pixels.
        let rgba = atlas();
        let w = TILE * TILES;
        let mask = |tile: u32| -> Vec<bool> {
            (0..TILE * TILE)
                .map(|i| {
                    let (x, y) = (i % TILE, i / TILE);
                    rgba[((y * w + tile * TILE + x) * 4 + 3) as usize] > 127
                })
                .collect()
        };

        for a in 0..u32::from(COLORS) {
            for b in 0..a {
                let diff = mask(a)
                    .iter()
                    .zip(mask(b))
                    .filter(|(p, q)| **p != *q)
                    .count();
                assert!(diff > 300, "tiles {a} and {b} differ in only {diff} pixels");
            }
        }
    }
}
