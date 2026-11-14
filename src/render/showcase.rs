//! The title screen's backdrop: gems drifting down behind the buttons, slow coloured glows
//! and twinkles. Everything is a function of the clock, so it keeps no state. Calm effects
//! freeze the gems where they are and leave out the glows and twinkles.

use std::f32::consts::TAU;

use game_toolkit::prelude::*;

use super::effects::Effects;
use super::fx::gem_color;
use super::gem_art::{TILE, TILES};
use super::layout::{DESIGN_H, DESIGN_W, Layout};

/// The coloured glows.
const GLOWS: Layer = Layer(1);
/// The drifting gems.
const GEMS: Layer = Layer(2);
/// The twinkles.
const TWINKLES: Layer = Layer(3);

/// How many gems drift past.
const GEM_COUNT: u32 = 28;
/// How many twinkles can be on screen.
const TWINKLE_COUNT: u32 = 36;

/// A number in [0, 1) from `i` and `k`, the same every frame.
fn hash(i: u32, k: u32) -> f32 {
    let mut x = i.wrapping_mul(0x9E37_79B9) ^ k.wrapping_mul(0x85EB_CA6B);
    x ^= x >> 16;
    x = x.wrapping_mul(0x7FEB_352D);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846C_A68B);
    x ^= x >> 16;
    (x >> 8) as f32 / (1u32 << 24) as f32
}

/// The six gem colours in a loop: `u` 0 is the first, 1 the next, blended in between.
pub fn rainbow(u: f32) -> Color {
    let u = u.rem_euclid(6.0);
    let i = u.floor() as u8;
    gem_color(i).lerp(gem_color((i + 1) % 6), u.fract())
}

/// Draws the backdrop at clock `time`.
pub fn backdrop(p: &mut Painter<'_>, l: &Layout, atlas: TextureId, time: f32, effects: Effects) {
    let full = effects.full();
    let t = if full { time } else { 0.0 };
    if full {
        glows(p, l, t);
        twinkles(p, l, t);
    }
    // Small, faint and slow ones first: they are further away.
    let mut order: Vec<u32> = (0..GEM_COUNT).collect();
    order.sort_by(|a, b| hash(*a, 1).total_cmp(&hash(*b, 1)));
    for i in order {
        let depth = hash(i, 1);
        let size = 30.0 + depth * 60.0;
        let speed = 14.0 + depth * 40.0;
        let span = DESIGN_H + 2.0 * size;
        let y = (hash(i, 2) * span + speed * t).rem_euclid(span) - size;
        let x = hash(i, 0) * (DESIGN_W + 60.0) - 30.0 + 20.0 * (t * 0.5 + hash(i, 3) * TAU).sin();
        let spin = hash(i, 4) * TAU + (hash(i, 5) - 0.5) * 1.4 * t;
        let tile = i % TILES;
        p.draw_sprite(
            atlas,
            l.px([x - size / 2.0, y - size / 2.0]),
            [l.len(size); 2],
        )
        .region([(tile * TILE) as f32, 0.0], [TILE as f32; 2])
        .rotation(spin)
        .color(Color::WHITE.with_alpha(0.25 + 0.4 * depth))
        .layer(GEMS);
    }
}

/// Large soft patches of gem colour wandering about.
fn glows(p: &mut Painter<'_>, l: &Layout, t: f32) {
    for i in 0..6u32 {
        let a = t * (0.07 + 0.05 * hash(i, 10)) + hash(i, 11) * TAU;
        let c = [
            DESIGN_W / 2.0 + 360.0 * a.cos(),
            DESIGN_H / 2.0 + 260.0 * (a * 1.3).sin(),
        ];
        let (center, r) = (l.px(c), l.len(280.0));
        let inner = gem_color(i as u8).with_alpha(0.22);
        let outer = inner.with_alpha(0.0);
        // A fan of slices fading from the middle out: no hard edge.
        const SLICES: u32 = 40;
        let rim = |k: u32| {
            let a = k as f32 / SLICES as f32 * TAU;
            [center[0] + r * a.cos(), center[1] + r * a.sin()]
        };
        for k in 0..SLICES {
            p.quad_colored(
                [center, center, rim(k + 1), rim(k)],
                [inner, inner, outer, outer],
                GLOWS,
            );
        }
    }
}

/// Four-pointed sparkles popping up all over, each somewhere new every time.
fn twinkles(p: &mut Painter<'_>, l: &Layout, t: f32) {
    for i in 0..TWINKLE_COUNT {
        let period = 1.6 + 2.0 * hash(i, 20);
        let u = t / period + hash(i, 21);
        let round = u.floor() as u32;
        let k = u.fract();
        // Visible for the first third of each round: grows, then shrinks.
        if k > 1.0 / 3.0 {
            continue;
        }
        let grow = 1.0 - (6.0 * k - 1.0).abs();
        let pos = [
            hash(i * 977 + round, 22) * DESIGN_W,
            hash(i * 977 + round, 23) * DESIGN_H,
        ];
        let color = if hash(i + round, 24) < 0.5 {
            Color::WHITE
        } else {
            rainbow(hash(i + round, 25) * 6.0)
        };
        let s = l.len((8.0 + 10.0 * hash(i + round, 26)) * grow);
        let c = l.px(pos);
        let w = l.len(1.8);
        let color = color.with_alpha(0.9);
        let add = BlendMode::Additive;
        p.line_ex([c[0] - s, c[1]], [c[0] + s, c[1]], w, color, TWINKLES, add);
        p.line_ex([c[0], c[1] - s], [c[0], c[1] + s], w, color, TWINKLES, add);
        p.circle_ex(c, w * 1.5, 0.0, color, TWINKLES);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_stays_in_range_and_varies() {
        let values: Vec<f32> = (0..1000).map(|i| hash(i, 7)).collect();
        assert!(values.iter().all(|v| (0.0..1.0).contains(v)));
        let mean = values.iter().sum::<f32>() / values.len() as f32;
        assert!((0.4..0.6).contains(&mean), "{mean}");
    }

    #[test]
    fn rainbow_loops_through_the_gem_colours() {
        assert_eq!(rainbow(0.0), gem_color(0));
        assert_eq!(rainbow(6.0), gem_color(0));
        assert_eq!(rainbow(-1.0), gem_color(5));
    }
}
