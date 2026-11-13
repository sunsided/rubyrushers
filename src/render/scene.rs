//! The background, the board and its gems.

use game_toolkit::prelude::*;

use super::board_view::{BoardView, Sprite};
use super::gem_art::{TILE, WILD_TILE};
use super::layout::{BOARD, BOARD_SIZE, CELL, Layout};
use crate::logic::{Pos, Power};

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

/// Draws the board, its gems and the marks; `time` drives the pulsing.
pub fn board(
    p: &mut Painter<'_>,
    layout: &Layout,
    view: &BoardView,
    atlas: TextureId,
    time: f32,
    marks: Marks,
) {
    let pad = 8.0;
    p.rect_ex(
        layout.px([BOARD[0] - pad, BOARD[1] - pad]),
        [layout.len(BOARD_SIZE + 2.0 * pad); 2],
        Color::rgba(0.0, 0.0, 0.0, 0.45),
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

    let pulse = 0.5 + 0.5 * (time * 6.0).sin();
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

    for s in view.sprites() {
        gem(p, layout, atlas, s, time, marks.selected);
    }
}

/// Window position of the centre of (fractional) cell `c`.
fn cell_px(layout: &Layout, c: [f32; 2]) -> [f32; 2] {
    layout.px(Layout::cell_center(c))
}

/// Draws one gem with its shadow and power marks.
fn gem(
    p: &mut Painter<'_>,
    layout: &Layout,
    atlas: TextureId,
    s: &Sprite,
    time: f32,
    selected: Option<Pos>,
) {
    // New gems fade in as they drop onto the board.
    let alpha = s.alpha * (s.pos[1] + 1.0).clamp(0.0, 1.0);
    if alpha <= 0.0 {
        return;
    }
    let is_selected = selected.is_some_and(|c| s.pos == [c.x as f32, c.y as f32]);
    let center = cell_px(layout, s.pos);
    let bob = if is_selected {
        1.0 + 0.06 * (time * 8.0).sin()
    } else {
        1.0
    };
    let size = layout.cell() * 0.94 * s.scale * bob;
    let tile = if s.gem.power == Power::Wild {
        WILD_TILE
    } else {
        u32::from(s.gem.color)
    };
    let rotation = if s.gem.power == Power::Wild {
        time * 1.8
    } else {
        0.0
    };
    let region = ([(tile * TILE) as f32, 0.0], [TILE as f32; 2]);
    let top_left = |c: [f32; 2]| [c[0] - size / 2.0, c[1] - size / 2.0];
    let off = layout.len(4.0);

    // Glows under special gems.
    let glow = match s.gem.power {
        Power::Bomb => Some(Color::rgb(1.0, 0.55, 0.15)),
        Power::Cross => Some(Color::rgb(0.6, 0.85, 1.0)),
        Power::Wild => Some(Color::WHITE),
        Power::None => None,
    };
    if let Some(g) = glow {
        let pulse = 0.5 + 0.5 * (time * 5.0 + s.gem.id as f32).sin();
        p.circle_ex(
            center,
            size * (0.5 + 0.06 * pulse),
            0.0,
            g.with_alpha(alpha * (0.25 + 0.25 * pulse)),
            CELLS,
        );
    }

    p.draw_sprite(
        atlas,
        top_left([center[0] + off, center[1] + off]),
        [size; 2],
    )
    .region(region.0, region.1)
    .rotation(rotation)
    .color(Color::BLACK.with_alpha(0.35 * alpha))
    .layer(GEMS);
    p.draw_sprite(atlas, top_left(center), [size; 2])
        .region(region.0, region.1)
        .rotation(rotation)
        .color(Color::WHITE.with_alpha(alpha))
        .layer(GEMS);

    match s.gem.power {
        Power::Bomb => {
            let r = size * (0.36 + 0.03 * (time * 7.0).sin());
            p.circle_ex(
                center,
                r,
                layout.len(3.0),
                Color::rgba(1.0, 0.9, 0.5, alpha * 0.9),
                MARKS,
            );
        }
        Power::Cross => {
            let l = size * 0.32;
            let w = layout.len(4.0);
            let c = Color::rgba(1.0, 1.0, 1.0, alpha * 0.85);
            p.line_ex(
                [center[0] - l, center[1]],
                [center[0] + l, center[1]],
                w,
                c,
                MARKS,
                BlendMode::Additive,
            );
            p.line_ex(
                [center[0], center[1] - l],
                [center[0], center[1] + l],
                w,
                c,
                MARKS,
                BlendMode::Additive,
            );
        }
        Power::Wild | Power::None => {}
    }
    if s.gem.time_bonus {
        p.circle_ex(
            center,
            size * 0.47,
            layout.len(3.0),
            Color::rgba(0.4, 1.0, 0.9, alpha * (0.6 + 0.4 * (time * 4.0).sin())),
            MARKS,
        );
        p.draw_text([center[0], center[1] + size * 0.12], "+5")
            .size(size * 0.3)
            .align(Align::Center)
            .color(Color::WHITE.with_alpha(alpha))
            .layer(MARKS);
    }
    if is_selected {
        p.circle_ex(
            center,
            layout.cell() * 0.5,
            layout.len(3.0),
            Color::rgba(1.0, 1.0, 1.0, 0.9),
            MARKS,
        );
    }
}
