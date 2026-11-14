//! Text and panels: the side panel while playing, the title and game-over screens.

use game_toolkit::prelude::*;

use super::effects::Effects;
use super::layout::{BOARD, BOARD_SIZE, DESIGN_H, DESIGN_W, Layout};
use super::showcase;
use crate::logic::mode::RUSH_SECS;
use crate::logic::mode::WARN_SECS;
use crate::logic::score;
use crate::logic::{Gravity, Mode, Session};

/// Layer of panels and their text, above everything on the board.
const HUD: Layer = Layer(6);
/// Accent colour for headings.
const ACCENT: Color = Color::rgb(1.0, 0.45, 0.5);
/// Muted colour for labels.
const MUTED: Color = Color::rgb(0.65, 0.68, 0.85);

/// The modes offered on the title screen, in button order.
pub const MODES: [Mode; 3] = [Mode::rush(), Mode::Endless, Mode::tilt()];

/// Design-space rectangle (top-left, size) of title button `i`.
#[must_use]
pub fn button_rect(i: usize) -> ([f32; 2], [f32; 2]) {
    let size = [460.0, 84.0];
    ([(DESIGN_W - size[0]) / 2.0, 340.0 + i as f32 * 100.0], size)
}

/// Which title button contains design point `d`.
#[must_use]
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
    effects: Effects,
) {
    gravity(p, l, session);

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
    speed(p, l, session, x);
    clock(p, l, session, x, time, effects);

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

    help(p, l, x, effects);
}

/// The speed bonus in the panel at `x`, with a bar for the time left to keep it.
fn speed(p: &mut Painter<'_>, l: &Layout, session: &Session, x: f32) {
    if session.speed() == 0 {
        return;
    }

    let label = format!("SPEED x{}", score::speed_factor(session.speed()));
    text(p, l, [x, 322.0], &label, 18.0, ACCENT, Align::Left);

    let w = BOARD[0] - x - 40.0;
    let left = session.speed_left();
    p.rect_ex(
        l.px([x, 346.0]),
        [l.len(w * left), l.len(5.0)],
        ACCENT,
        HUD,
        BlendMode::Alpha,
    );
}

/// The clock in the panel at `x` (modes with one): a bar, the seconds left, and what the
/// next time gem is worth. With full effects the last ten seconds flash.
fn clock(p: &mut Painter<'_>, l: &Layout, session: &Session, x: f32, time: f32, effects: Effects) {
    let Some(remaining) = session.mode().remaining() else {
        return;
    };

    text(p, l, [x, 360.0], "TIME", 18.0, MUTED, Align::Left);

    let w = BOARD[0] - x - 40.0;
    let frac = (remaining / RUSH_SECS).clamp(0.0, 1.0);
    let urgent = remaining < 10.0;
    let flash = if effects.full() {
        0.5 + 0.5 * (time * 10.0).sin()
    } else {
        0.0
    };

    let color = if urgent {
        Color::rgb(1.0, 0.3, 0.25).lerp(Color::WHITE, 0.3 * flash)
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

    let next = session.next_time_bonus();
    let rule = session.time_rule().name();
    let gems = if next > 0.0 {
        format!("time gems +{} s ({rule})", (next * 10.0).round() / 10.0)
    } else {
        format!("time gems spent ({rule})")
    };

    text(p, l, [x, 452.0], &gems, 14.0, MUTED, Align::Left);
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

/// The effect level and the controls, at the bottom of the panel at `x`.
fn help(p: &mut Painter<'_>, l: &Layout, x: f32, effects: Effects) {
    text(
        p,
        l,
        [x, DESIGN_H - 84.0],
        &format!("F2 effects: {}", effects.name()),
        14.0,
        MUTED,
        Align::Left,
    );
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

/// The title screen: the name and one button per mode. With full effects the name's
/// letters bob and run through the gem colours, and the button frames shimmer.
pub fn title(
    p: &mut Painter<'_>,
    l: &Layout,
    bests: &crate::scores::Bests,
    hover: Option<usize>,
    time: f32,
    effects: Effects,
) {
    let cx = DESIGN_W / 2.0;
    title_name(p, l, time, effects);

    text(
        p,
        l,
        [cx, 210.0],
        "Line up three or more. Bigger lines make special gems.",
        20.0,
        MUTED,
        Align::Center,
    );

    for (i, &best) in bests.iter().enumerate() {
        button(p, l, i, best, hover == Some(i), time, effects);
    }
}

/// The game's name on the title screen; with full effects, letter by letter, bobbing and
/// running through the gem colours.
fn title_name(p: &mut Painter<'_>, layout: &Layout, time: f32, effects: Effects) {
    let cx = DESIGN_W / 2.0;
    let name = "RUBY RUSHERS";
    if !effects.full() {
        text(p, layout, [cx, 120.0], name, 72.0, ACCENT, Align::Center);
        return;
    }

    // Letter by letter, each placed after the width of the text before it.
    let px = layout.len(72.0);
    let width = p.measure_text(name, px)[0];
    let left = layout.px([cx, 0.0])[0] - width / 2.0;

    for (j, ch) in name.char_indices() {
        let x = left + p.measure_text(&name[..j], px)[0];
        let k = j as f32;
        let y = layout.px([0.0, 120.0 + 7.0 * (time * 3.0 - k * 0.45).sin()])[1];
        p.draw_text([x, y], &ch.to_string())
            .size(px)
            .color(showcase::rainbow(time * 1.2 - k * 0.35))
            .align(Align::Left)
            .layer(HUD);
    }
}

/// Title button `i`: the mode's name, its blurb and its `best` score; `lit` under the
/// pointer. With full effects the frame shimmers.
fn button(
    p: &mut Painter<'_>,
    l: &Layout,
    i: usize,
    best: u64,
    lit: bool,
    time: f32,
    effects: Effects,
) {
    let blurbs = [
        "60 seconds, time gems add more",
        "No clock, ends when you're stuck",
        "60 seconds, gravity turns every 15",
    ];

    let cx = DESIGN_W / 2.0;
    let (pos, size) = button_rect(i);
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
        if effects.full() {
            showcase::rainbow(time * 0.8 + i as f32 * 2.0)
        } else {
            ACCENT
        },
        HUD,
        BlendMode::Alpha,
    );

    text(
        p,
        l,
        [cx, pos[1] + 12.0],
        &format!("{}  {}", i + 1, MODES[i].name()),
        32.0,
        Color::WHITE,
        Align::Center,
    );

    let sub = if best > 0 {
        format!("{}  ·  best {}", blurbs[i], best)
    } else {
        blurbs[i].to_owned()
    };

    text(p, l, [cx, pos[1] + 54.0], &sub, 15.0, MUTED, Align::Center);
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
        Mode::Rush { .. } | Mode::Tilt { .. } => "Time's up!",
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

/// A filled arrow head at design point `c`, pointing along `dir`, `size` long.
fn arrow(p: &mut Painter<'_>, l: &Layout, c: [f32; 2], dir: [f32; 2], size: f32, color: Color) {
    let (fx, fy) = (dir[0] * size / 2.0, dir[1] * size / 2.0);
    // Perpendicular half-width.
    let (sx, sy) = (-dir[1] * size / 2.0, dir[0] * size / 2.0);
    let pts = [
        l.px([c[0] + fx, c[1] + fy]),
        l.px([c[0] - fx + sx, c[1] - fy + sy]),
        l.px([c[0] - fx * 0.4, c[1] - fy * 0.4]),
        l.px([c[0] - fx - sx, c[1] - fy - sy]),
    ];
    p.polygon_ex(&pts, color, HUD, BlendMode::Alpha);
}

/// The gravity readout in the panel (tilt mode): which way gems fall and when it turns.
pub fn gravity(p: &mut Painter<'_>, l: &Layout, session: &Session) {
    let Some(turn_in) = session.mode().turn_in() else {
        return;
    };

    let x = 40.0;
    text(p, l, [x, 520.0], "GRAVITY", 18.0, MUTED, Align::Left);
    arrow(
        p,
        l,
        [x + 22.0, 570.0],
        session.gravity().dir(),
        36.0,
        Color::WHITE,
    );

    let next = if turn_in > 0.0 {
        format!("turns in {:.0} s", turn_in.ceil())
    } else {
        "turning...".to_owned()
    };
    text(p, l, [x + 56.0, 556.0], &next, 18.0, MUTED, Align::Left);
}

/// Big arrows over the board pointing where gravity turns next, shown shortly before.
pub fn tilt_warning(
    p: &mut Painter<'_>,
    l: &Layout,
    session: &Session,
    time: f32,
    effects: Effects,
) {
    let Some(turn_in) = session.mode().turn_in() else {
        return;
    };
    if turn_in > WARN_SECS || session.is_over() {
        return;
    }

    let next: Gravity = session.gravity().turned();
    let blink = if effects.full() {
        0.5 + 0.5 * (time * 12.0).sin()
    } else {
        1.0
    };
    let color = Color::rgba(1.0, 0.85, 0.3, 0.25 + 0.35 * blink);
    let mid = [BOARD[0] + BOARD_SIZE / 2.0, BOARD[1] + BOARD_SIZE / 2.0];
    let d = next.dir();
    for k in [-1.0, 0.0, 1.0] {
        let c = [mid[0] + d[0] * k * 120.0, mid[1] + d[1] * k * 120.0];
        arrow(p, l, c, d, 110.0, color);
    }
}
