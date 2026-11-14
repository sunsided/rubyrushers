//! The background, the board and its gems.

use game_toolkit::prelude::*;

use super::effects::Effects;
use super::fx::Fx;
use super::fx::gem_color;
use super::gem_art::{TILE, WILD_TILE};
use super::layout::{BOARD, BOARD_SIZE, CELL, Layout};
use super::showcase::rainbow;
use crate::logic::score::MAX_SPEED;
use crate::logic::{Gem, Gravity, Pos, Power, SIZE, Session};

/// The window background.
const BG: Layer = Layer(0);
/// Board cells and glows under the gems.
const CELLS: Layer = Layer(1);
/// The gems.
const GEMS: Layer = Layer(2);
/// Marks on top of the gems.
const MARKS: Layer = Layer(3);

/// What is highlighted on the board.
#[derive(Clone, Copy, Debug, Default)]
pub struct Marks {
    /// The gem the player picked.
    pub selected: Option<Pos>,
    /// A move to suggest.
    pub hint: Option<(Pos, Pos)>,
}

/// Fills the window with the background gradient.
pub fn background(p: &mut Painter<'_>, w: f32, h: f32) {
    let top = Color::rgb(0.10, 0.06, 0.18);
    let bottom = Color::rgb(0.03, 0.05, 0.12);
    p.quad_colored(
        [[0.0, 0.0], [w, 0.0], [w, h], [0.0, h]],
        [top, top, bottom, bottom],
        BG,
    );
}

/// A disc of `color` at window point `center` fading out to nothing at radius `r`.
pub fn soft_disc(p: &mut Painter<'_>, center: [f32; 2], r: f32, color: Color, layer: Layer) {
    /// Slices of the fan.
    const SLICES: u32 = 32;
    let rim = |k: u32| {
        let a = k as f32 / SLICES as f32 * std::f32::consts::TAU;
        [center[0] + r * a.cos(), center[1] + r * a.sin()]
    };
    let outer = color.with_alpha(0.0);
    for k in 0..SLICES {
        p.quad_colored(
            [center, center, rim(k + 1), rim(k)],
            [color, color, outer, outer],
            layer,
        );
    }
}

/// One gem as it is to be drawn.
#[derive(Clone, Copy, Debug)]
struct Look {
    /// The gem.
    gem: Gem,
    /// Position in cells.
    pos: [f32; 2],
    /// Size factor.
    scale: f32,
    /// Opacity.
    alpha: f32,
    /// Rotation in radians.
    spin: f32,
    /// Stretch factor along the fall (falling gems, full effects).
    stretch: f32,
    /// Gems fall sideways: stretch horizontally.
    sideways: bool,
    /// Seconds a time gem adds now (0: worth nothing, drawn as a plain gem).
    bonus: f32,
    /// How strongly the passing shine lights it up (full effects), 0 to 1.
    shine: f32,
}

/// Draws the board, its gems, the gems being cleared and the marks; `time` drives the
/// pulsing.
#[allow(clippy::too_many_arguments, reason = "plain inputs of one draw call")]
pub fn board(
    p: &mut Painter<'_>,
    layout: &Layout,
    session: &Session,
    fx: &Fx,
    atlas: TextureId,
    time: f32,
    marks: Marks,
    effects: Effects,
) {
    let full = effects.full();
    let pad = 8.0;
    if full {
        frame(p, layout, session, time, pad);
    }
    // Opaque enough that nothing behind the board looks like a gem between them.
    p.rect_ex(
        layout.px([BOARD[0] - pad, BOARD[1] - pad]),
        [layout.len(BOARD_SIZE + 2.0 * pad); 2],
        Color::rgba(0.0, 0.0, 0.0, if full { 0.8 } else { 0.45 }),
        CELLS,
        BlendMode::Alpha,
    );
    for c in Pos::all() {
        let shade = if (c.x + c.y) % 2 == 0 { 0.07 } else { 0.03 };
        p.rect_ex(
            layout.px([BOARD[0] + c.x as f32 * CELL, BOARD[1] + c.y as f32 * CELL]),
            [layout.cell(); 2],
            Color::rgba(0.7, 0.75, 1.0, shade),
            CELLS,
            BlendMode::Alpha,
        );
    }

    let pulse = if full {
        0.5 + 0.5 * (time * 6.0).sin()
    } else {
        0.6
    };
    if let Some((a, b)) = marks.hint {
        for c in [a, b] {
            p.circle_ex(
                cell_px(layout, [c.x as f32, c.y as f32]),
                layout.cell() * (0.45 + 0.05 * pulse),
                0.0,
                Color::rgba(1.0, 1.0, 0.8, 0.12 + 0.18 * pulse),
                CELLS,
            );
        }
    }

    let sideways = matches!(session.gravity(), Gravity::Left | Gravity::Right);
    let bonus = session.next_time_bonus();
    for q in session.pieces() {
        let selected = marks
            .selected
            .is_some_and(|c| q.pos == [c.x as f32, c.y as f32]);
        let wobble = if full && selected {
            0.12 * (time * 9.0).sin()
        } else {
            0.0
        };
        let spin = if full && q.gem.power == Power::Wild {
            time * 1.8
        } else {
            wobble
        };
        let stretch = if full {
            1.0 + (q.speed * 0.004).min(0.14) - 0.2 * fx.squash(q.gem.id)
        } else {
            1.0
        };
        // Resting gems breathe a little, each in its own rhythm.
        let breath = if full && q.speed == 0.0 && !q.swapping {
            1.0 + 0.025 * (time * 2.2 + q.gem.id as f32 * 1.7).sin()
        } else {
            1.0
        };
        let look = Look {
            gem: q.gem,
            pos: q.pos,
            scale: fx.born_scale(q.gem.id) * breath,
            alpha: 1.0,
            spin,
            stretch,
            sideways,
            bonus,
            shine: if full { shine(q.pos, time) } else { 0.0 },
        };
        gem(p, layout, atlas, look, time, full);
        if selected {
            p.circle_ex(
                cell_px(layout, q.pos),
                layout.cell() * 0.5,
                layout.len(3.0),
                Color::rgba(1.0, 1.0, 1.0, 0.9),
                MARKS,
            );
        }
    }
    for v in fx.vanishing() {
        let look = Look {
            gem: v.gem,
            pos: v.pos,
            scale: v.scale(),
            alpha: v.alpha(),
            spin: if full { v.spin() } else { 0.0 },
            stretch: 1.0,
            sideways: false,
            bonus,
            shine: 0.0,
        };
        gem(p, layout, atlas, look, time, full);
    }
}

/// How brightly the shine sweeping diagonally across the board every few seconds lights
/// up a gem at cell `c`, 0 to 1.
fn shine(c: [f32; 2], time: f32) -> f32 {
    /// Seconds between sweeps.
    const EVERY: f32 = 6.0;
    // From before the top-left corner (-0.2) to past the bottom-right (1.2).
    let front = (time / EVERY).fract() * 2.5 - 0.2;
    let at = (c[0] + c[1]) / (2.0 * (SIZE - 1) as f32);
    (1.0 - (front - at).abs() * 7.0).max(0.0)
}

/// The glowing frame around the board, running through the gem colours; it burns
/// brighter with the speed streak.
fn frame(p: &mut Painter<'_>, layout: &Layout, session: &Session, time: f32, pad: f32) {
    let heat = session.speed() as f32 / MAX_SPEED as f32;
    let [x0, y0] = [BOARD[0] - pad, BOARD[1] - pad];
    let [x1, y1] = [x0 + BOARD_SIZE + 2.0 * pad, y0 + BOARD_SIZE + 2.0 * pad];
    let corners = [[x0, y0], [x1, y0], [x1, y1], [x0, y1]];
    // Colours flow around the frame: each corner a quarter further along.
    let color = |i: usize| rainbow(time * (0.6 + 1.5 * heat) + i as f32 * 1.5);
    let width = 4.0 + 3.0 * heat;
    let glow = 14.0 + 22.0 * heat;
    let glow_alpha = 0.18 + 0.27 * heat;
    for i in 0..4 {
        let (a, b) = (corners[i], corners[(i + 1) % 4]);
        let (ca, cb) = (color(i), color(i + 1));
        // Outward normal of this edge (the frame runs clockwise).
        let len = (b[0] - a[0]).hypot(b[1] - a[1]);
        let n = [(b[1] - a[1]) / len, -(b[0] - a[0]) / len];
        let out = |q: [f32; 2], d: f32| layout.px([q[0] + n[0] * d, q[1] + n[1] * d]);
        let edge_glow = |c: Color| c.with_alpha(glow_alpha);
        // Soft glow outward, then the bright band.
        p.quad_colored(
            [out(a, 0.0), out(b, 0.0), out(b, glow), out(a, glow)],
            [
                edge_glow(ca),
                edge_glow(cb),
                cb.with_alpha(0.0),
                ca.with_alpha(0.0),
            ],
            CELLS,
        );
        p.quad_colored(
            [out(a, 0.0), out(b, 0.0), out(b, width), out(a, width)],
            [ca, cb, cb, ca],
            CELLS,
        );
        // Round the glow off around the corner; the board covers the inner part.
        soft_disc(
            p,
            layout.px(a),
            layout.len(glow),
            ca.with_alpha(glow_alpha),
            CELLS,
        );
    }
}

/// Window position of the centre of (fractional) cell `c`.
fn cell_px(layout: &Layout, c: [f32; 2]) -> [f32; 2] {
    layout.px(Layout::cell_center(c))
}

/// Draws one gem with its shadow and power marks; `full` lets the marks pulse.
fn gem(p: &mut Painter<'_>, layout: &Layout, atlas: TextureId, s: Look, time: f32, full: bool) {
    // New gems fade in as they come onto the board over its edge.
    let last = (SIZE - 1) as f32;
    let outside = [-s.pos[0], s.pos[0] - last, -s.pos[1], s.pos[1] - last]
        .into_iter()
        .fold(0.0f32, f32::max);
    let alpha = s.alpha * (1.0 - outside).clamp(0.0, 1.0);
    if alpha <= 0.0 || s.scale <= 0.0 {
        return;
    }
    let center = cell_px(layout, s.pos);
    let size = layout.cell() * 0.94 * s.scale;
    let (across, along) = (size / s.stretch.sqrt(), size * s.stretch);
    let dims = if s.sideways {
        [along, across]
    } else {
        [across, along]
    };
    let tile = if s.gem.power == Power::Wild {
        WILD_TILE
    } else {
        u32::from(s.gem.color)
    };
    let region = ([(tile * TILE) as f32, 0.0], [TILE as f32; 2]);
    let top_left = |c: [f32; 2]| [c[0] - dims[0] / 2.0, c[1] - dims[1] / 2.0];
    let off = layout.len(4.0);
    let wave = |speed: f32, phase: f32| {
        if full {
            (time * speed + phase).sin()
        } else {
            0.0
        }
    };

    // Glows under special gems.
    let glow = match s.gem.power {
        Power::Bomb => Some(Color::rgb(1.0, 0.55, 0.15)),
        Power::Cross => Some(Color::rgb(0.6, 0.85, 1.0)),
        Power::Wild => Some(Color::WHITE),
        Power::None => None,
    };
    if full && s.gem.power != Power::Wild {
        // A faint pool of the gem's own colour beneath it, kept inside its cell so the
        // gaps between gems stay dark.
        soft_disc(
            p,
            center,
            size * 0.5,
            gem_color(s.gem.color).with_alpha(0.14 * alpha),
            CELLS,
        );
    }
    if let Some(g) = glow {
        let pulse = 0.5 + 0.5 * wave(5.0, s.gem.id as f32);
        p.circle_ex(
            center,
            size * (0.5 + 0.06 * pulse),
            0.0,
            g.with_alpha(alpha * (0.25 + 0.25 * pulse)),
            CELLS,
        );
    }

    p.draw_sprite(atlas, top_left([center[0] + off, center[1] + off]), dims)
        .region(region.0, region.1)
        .rotation(s.spin)
        .color(Color::BLACK.with_alpha(0.35 * alpha))
        .layer(GEMS);
    p.draw_sprite(atlas, top_left(center), dims)
        .region(region.0, region.1)
        .rotation(s.spin)
        .color(Color::WHITE.with_alpha(alpha))
        .layer(GEMS);
    if s.shine > 0.0 {
        // The gem once more, added on top: it lights up in its own colour.
        p.draw_sprite(atlas, top_left(center), dims)
            .region(region.0, region.1)
            .rotation(s.spin)
            .color(Color::WHITE.with_alpha(0.55 * s.shine * alpha))
            .blend(BlendMode::Additive)
            .layer(GEMS);
    }

    match s.gem.power {
        Power::Bomb => {
            let r = size * (0.36 + 0.03 * wave(7.0, 0.0));
            let c = Color::rgba(1.0, 0.9, 0.5, alpha * 0.9);
            p.circle_ex(center, r, layout.len(3.0), c, MARKS);
        }
        Power::Cross => {
            let l = size * 0.32;
            let w = layout.len(4.0);
            let c = Color::rgba(1.0, 1.0, 1.0, alpha * 0.85);
            let add = BlendMode::Additive;
            p.line_ex(
                [center[0] - l, center[1]],
                [center[0] + l, center[1]],
                w,
                c,
                MARKS,
                add,
            );
            p.line_ex(
                [center[0], center[1] - l],
                [center[0], center[1] + l],
                w,
                c,
                MARKS,
                add,
            );
        }
        Power::Wild | Power::None => {}
    }
    if s.gem.time_bonus && s.bonus > 0.0 {
        let c = Color::rgba(0.4, 1.0, 0.9, alpha * (0.7 + 0.3 * wave(4.0, 0.0)));
        p.circle_ex(center, size * 0.47, layout.len(3.0), c, MARKS);
        let label = format!("+{}", (s.bonus * 10.0).round() / 10.0);
        p.draw_text([center[0], center[1] + size * 0.12], &label)
            .size(size * 0.3)
            .align(Align::Center)
            .color(Color::WHITE.with_alpha(alpha))
            .layer(MARKS);
    }
}
