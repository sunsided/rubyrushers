//! Text and panels: the side panel while playing, the title and game-over screens.

use game_toolkit::prelude::*;

use super::layout::{BOARD, DESIGN_H, DESIGN_W, Layout};
use crate::logic::mode::RUSH_SECS;
use crate::logic::{Mode, Session};

/// Layer of panels and their text, above everything on the board.
const HUD: Layer = Layer(6);
/// Accent colour for headings.
const ACCENT: Color = Color::rgb(1.0, 0.45, 0.5);
/// Muted colour for labels.
const MUTED: Color = Color::rgb(0.65, 0.68, 0.85);

/// The modes offered on the title screen, in button order.
pub const MODES: [Mode; 2] = [Mode::rush(), Mode::Endless];

/// Design-space rectangle (top-left, size) of title button `i`.
pub fn button_rect(i: usize) -> ([f32; 2], [f32; 2]) {
    let size = [300.0, 84.0];
    ([(DESIGN_W - size[0]) / 2.0, 360.0 + i as f32 * 110.0], size)
}

/// Which title button contains design point `d`.
pub fn button_at(d: [f32; 2]) -> Option<usize> {
    (0..MODES.len()).find(|&i| {
        let (pos, size) = button_rect(i);
        d[0] >= pos[0] && d[0] < pos[0] + size[0] && d[1] >= pos[1] && d[1] < pos[1] + size[1]
    })
}

/// Text of `size` design units at design point `pos`.
fn text(
    p: &mut Painter<'_>,
    l: &Layout,
    pos: [f32; 2],
    s: &str,
    size: f32,
    color: Color,
    align: Align,
) {
    p.draw_text(l.px(pos), s)
        .size(l.len(size))
        .color(color)
        .align(align)
        .layer(HUD);
}

/// The side panel: name, mode, score, clock.
pub fn panel(
    p: &mut Painter<'_>,
    l: &Layout,
    session: &Session,
    shown_score: u64,
    best: u64,
    time: f32,
) {
    let x = 40.0;
    text(p, l, [x, 40.0], "RUBY", 44.0, ACCENT, Align::Left);
    text(p, l, [x, 84.0], "RUSHERS", 44.0, Color::WHITE, Align::Left);
    text(
        p,
        l,
        [x, 150.0],
        session.mode().name(),
        22.0,
        MUTED,
        Align::Left,
    );

    text(p, l, [x, 210.0], "SCORE", 18.0, MUTED, Align::Left);
    text(
        p,
        l,
        [x, 232.0],
        &shown_score.to_string(),
        46.0,
        Color::WHITE,
        Align::Left,
    );
    text(
        p,
        l,
        [x, 300.0],
        &format!("Best  {best}"),
        18.0,
        MUTED,
        Align::Left,
    );

    if let Mode::Rush { remaining } = *session.mode() {
        text(p, l, [x, 360.0], "TIME", 18.0, MUTED, Align::Left);
        let w = BOARD[0] - x - 40.0;
        let frac = (remaining / RUSH_SECS).clamp(0.0, 1.0);
        let urgent = remaining < 10.0;
        let color = if urgent {
            Color::rgb(1.0, 0.3, 0.25).lerp(Color::WHITE, 0.3 * (0.5 + 0.5 * (time * 10.0).sin()))
        } else {
            Color::rgb(0.4, 1.0, 0.85)
        };
        p.rect_ex(
            l.px([x, 388.0]),
            [l.len(w), l.len(18.0)],
            Color::rgba(1.0, 1.0, 1.0, 0.1),
            HUD,
            BlendMode::Alpha,
        );
        p.rect_ex(
            l.px([x, 388.0]),
            [l.len(w * frac), l.len(18.0)],
            color,
            HUD,
            BlendMode::Alpha,
        );
        text(
            p,
            l,
            [x, 414.0],
            &format!("{:.0} s", remaining.ceil()),
            30.0,
            color,
            Align::Left,
        );
    }
    if session.best_cascade() >= 2 {
        text(
            p,
            l,
            [x, 480.0],
            &format!("Best cascade x{}", session.best_cascade()),
            18.0,
            MUTED,
            Align::Left,
        );
    }
    text(
        p,
        l,
        [x, DESIGN_H - 60.0],
        "Swap neighbours: click both",
        14.0,
        MUTED,
        Align::Left,
    );
    text(
        p,
        l,
        [x, DESIGN_H - 40.0],
        "or drag. Esc: menu",
        14.0,
        MUTED,
        Align::Left,
    );
}

/// Dims the whole window.
fn dim(p: &mut Painter<'_>, w: f32, h: f32, alpha: f32) {
    p.rect_ex(
        [0.0, 0.0],
        [w, h],
        Color::rgba(0.02, 0.02, 0.06, alpha),
        HUD,
        BlendMode::Alpha,
    );
}

/// The title screen over a dimmed board: name and one button per mode.
pub fn title(
    p: &mut Painter<'_>,
    l: &Layout,
    w: f32,
    h: f32,
    bests: &[u64; 2],
    hover: Option<usize>,
    time: f32,
) {
    dim(p, w, h, 0.95);
    let cx = DESIGN_W / 2.0;
    let wobble = 4.0 * (time * 2.0).sin();
    text(
        p,
        l,
        [cx, 120.0 + wobble],
        "RUBY RUSHERS",
        72.0,
        ACCENT,
        Align::Center,
    );
    text(
        p,
        l,
        [cx, 210.0],
        "Line up three or more. Bigger lines make special gems.",
        20.0,
        MUTED,
        Align::Center,
    );
    let blurbs = [
        "60 seconds, time gems add more",
        "No clock, ends when you're stuck",
    ];
    for (i, mode) in MODES.iter().enumerate() {
        let (pos, size) = button_rect(i);
        let lit = hover == Some(i);
        let fill = if lit {
            Color::rgb(0.42, 0.16, 0.24)
        } else {
            Color::rgb(0.12, 0.09, 0.2)
        };
        p.rect_ex(
            l.px(pos),
            [l.len(size[0]), l.len(size[1])],
            fill,
            HUD,
            BlendMode::Alpha,
        );
        p.rect_outline_ex(
            l.px(pos),
            [l.len(size[0]), l.len(size[1])],
            l.len(2.0),
            ACCENT,
            HUD,
            BlendMode::Alpha,
        );
        text(
            p,
            l,
            [cx, pos[1] + 12.0],
            &format!("{}  {}", i + 1, mode.name()),
            32.0,
            Color::WHITE,
            Align::Center,
        );
        let sub = if bests[i] > 0 {
            format!("{}  ·  best {}", blurbs[i], bests[i])
        } else {
            blurbs[i].to_owned()
        };
        text(p, l, [cx, pos[1] + 54.0], &sub, 15.0, MUTED, Align::Center);
    }
}

/// The game-over card over the board.
pub fn game_over(
    p: &mut Painter<'_>,
    l: &Layout,
    w: f32,
    h: f32,
    session: &Session,
    best: u64,
    new_best: bool,
) {
    dim(p, w, h, 0.85);
    let cx = DESIGN_W / 2.0;
    let heading = match session.mode() {
        Mode::Rush { .. } => "Time's up!",
        Mode::Endless => "No moves left",
    };
    text(p, l, [cx, 170.0], heading, 56.0, ACCENT, Align::Center);
    text(p, l, [cx, 270.0], "SCORE", 20.0, MUTED, Align::Center);
    text(
        p,
        l,
        [cx, 296.0],
        &session.score().to_string(),
        72.0,
        Color::WHITE,
        Align::Center,
    );
    let best_line = if new_best {
        "New best!".to_owned()
    } else {
        format!("Best {best}")
    };
    text(
        p,
        l,
        [cx, 390.0],
        &best_line,
        26.0,
        Color::rgb(1.0, 0.85, 0.3),
        Align::Center,
    );
    if session.best_cascade() >= 2 {
        text(
            p,
            l,
            [cx, 432.0],
            &format!("Best cascade x{}", session.best_cascade()),
            20.0,
            MUTED,
            Align::Center,
        );
    }
    text(
        p,
        l,
        [cx, 520.0],
        "Click or press Enter",
        20.0,
        MUTED,
        Align::Center,
    );
}
