//! The window `--watch` opens: what the bot sees and thinks, live.
//!
//! Three boards side by side: the pixels it captured (**Sees**), the tiles it read from
//! them (**Reads**: solid where it trusts them, hollow where it does not yet, `?` where it
//! cannot tell, an orange frame where its own moves still work), and the board it expects
//! after its move (**Expects**: `?` where new gems will drop in, the next move it is
//! setting up in cyan). Below, the moves it weighed and how long each step takes.

use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::Ordering;

use anyhow::Result;
use game_toolkit::prelude::{
    Align, AppConfig, BlendMode, Color, Context, Frame, Game, Layer, Painter, TextureId,
};

use crate::board::SIZE;
use crate::insight::{self, Insight, Plan};
use crate::read::{HUES, WHITE};
use crate::solver::{Swap, Value};
use crate::tile::Tile;

/// Design width; everything scales to fit the window.
const W: f32 = 1280.0;
/// Design height.
const H: f32 = 800.0;
/// Side of each of the three boards.
const SIDE: f32 = 380.0;
/// Top of the boards.
const TOP: f32 = 96.0;
/// Left edges of the three boards.
const LEFTS: [f32; 3] = [30.0, 450.0, 870.0];
/// Shapes.
const SHAPES: Layer = Layer(1);
/// Marks over the shapes.
const MARKS: Layer = Layer(2);
/// Text.
const TEXT: Layer = Layer(3);
/// Muted text.
const MUTED: Color = Color::rgb(0.6, 0.62, 0.75);
/// The colour of the move the bot made.
const MOVE: Color = Color::WHITE;
/// The colour of the move it sets up.
const NEXT: Color = Color::rgb(0.3, 0.9, 1.0);
/// The colour of cells its own moves keep busy.
const BUSY: Color = Color::rgb(1.0, 0.55, 0.15);

/// Where to save a picture of the window once the bot is done (`--watch-shot`); the
/// window then closes.
static SHOT: Mutex<Option<PathBuf>> = Mutex::new(None);

/// Opens the window and shows the bot's rounds until it is closed, or, with `shot`, until
/// the bot is done and the window saved a picture of itself there.
///
/// # Errors
///
/// When the window or its graphics context cannot be created.
///
/// # Panics
///
/// When another thread panicked while holding the screenshot path.
pub fn show(shot: Option<PathBuf>) -> Result<()> {
    *SHOT.lock().expect("not poisoned") = shot;

    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the design size is a whole, positive number of pixels"
    )]
    let config = AppConfig {
        title: "RubyRushers bot: what it sees and thinks".into(),
        width: W as u32,
        height: H as u32,
        msaa_samples: 4,
        ..Default::default()
    };

    game_toolkit::prelude::run::<Viewer>(config).map_err(|e| anyhow::anyhow!("viewer: {e}"))
}

/// The viewer's state.
struct Viewer {
    /// The latest round.
    shown: Option<Insight>,
    /// The last move made, kept while rounds without a move go by.
    plan: Option<Plan>,
    /// The captured pixels as a texture, and its size.
    crop: Option<(TextureId, [usize; 2])>,
    /// The picture is taken: close at the next update.
    quit_next: bool,
}

impl Game for Viewer {
    fn init(_ctx: &mut Context) -> game_toolkit::prelude::Result<Self> {
        Ok(Self {
            shown: None,
            plan: None,
            crop: None,
            quit_next: false,
        })
    }

    fn update(&mut self, ctx: &mut Context, _dt: f32) {
        if self.quit_next {
            ctx.quit();
            return;
        }

        if insight::FINISHED.load(Ordering::Relaxed)
            && let Some(path) = SHOT.lock().expect("not poisoned").take()
        {
            ctx.gfx.request_screenshot(path);
            self.quit_next = true;
        }

        let Some(new) = insight::LATEST
            .lock()
            .expect("the bot does not panic")
            .take()
        else {
            return;
        };

        let img = &new.crop;
        let rgba: Vec<u8> = img
            .rgb
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|&[r, g, b]| [r, g, b, 255])
            .collect();

        let size = [img.width, img.height];
        match self.crop {
            Some((tex, s)) if s == size => ctx.gfx.update_texture_rgba(tex, &rgba),
            _ => {
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "a board crop is a few hundred pixels wide"
                )]
                let tex = ctx.gfx.create_texture_rgba(
                    size[0] as u32,
                    size[1] as u32,
                    &rgba,
                    Some("crop"),
                );
                self.crop = Some((tex, size));
            }
        }

        if new.plan.is_some() {
            self.plan.clone_from(&new.plan);
        }

        self.shown = Some(new);
    }

    fn render(&mut self, ctx: &mut Context, frame: &mut Frame) {
        let (width, height) = ctx.gfx.size();
        let v = View::fit(width as f32, height as f32);
        let mut p = frame.painter(&mut ctx.gfx);

        p.rect_ex(
            [0.0, 0.0],
            [width as f32, height as f32],
            Color::rgb(0.06, 0.05, 0.1),
            Layer(0),
            BlendMode::Alpha,
        );

        let Some(i) = &self.shown else {
            v.text(
                &mut p,
                [W / 2.0, H / 2.0],
                "waiting for the bot to find the board ...",
                24.0,
                MUTED,
                Align::Center,
            );
            return;
        };

        let status = if insight::FINISHED.load(Ordering::Relaxed) {
            format!("done: {} moves in {:.0} s", i.moves, i.elapsed)
        } else {
            i.status.clone()
        };

        headings(&mut p, &v, &status);

        // Sees: the captured board.
        if let Some((tex, _)) = self.crop {
            let g = i.view;
            let side = g.pitch * SIZE as f32;
            p.draw_sprite(tex, v.px([LEFTS[0], TOP]), [v.len(SIDE); 2])
                .region(g.origin, [side, side])
                .layer(SHAPES);
        }

        reads(&mut p, &v, i);

        if let Some(plan) = &self.plan {
            expects(&mut p, &v, plan);
        }

        candidates(&mut p, &v, self.plan.as_ref());
        stats(&mut p, &v, i);
    }
}

/// The window title, the status, and the title and legend above each board.
fn headings(p: &mut Painter<'_>, v: &View, status: &str) {
    v.text(
        p,
        [30.0, 22.0],
        "What the bot sees and thinks",
        28.0,
        Color::WHITE,
        Align::Left,
    );

    v.text(p, [W - 30.0, 28.0], status, 20.0, MUTED, Align::Right);

    let titles = [
        ("Sees", "a screenshot of the board"),
        ("Reads", "solid: trusted   hollow: not yet   ?: unreadable"),
        ("Expects", "after its move settles   ?: gems still to come"),
    ];
    for (k, (title, note)) in titles.iter().enumerate() {
        let x = LEFTS[k];
        v.text(p, [x, TOP - 34.0], title, 22.0, Color::WHITE, Align::Left);

        v.text(p, [x + 90.0, TOP - 28.0], note, 13.0, MUTED, Align::Left);

        v.frame(p, [x, TOP]);
    }
}

/// **Reads**: the tiles, trusted or not, and the busy cells.
fn reads(p: &mut Painter<'_>, v: &View, i: &Insight) {
    let cell = SIDE / SIZE as f32;
    for y in 0..SIZE {
        for x in 0..SIZE {
            let at = [LEFTS[1] + x as f32 * cell, TOP + y as f32 * cell];
            v.tile(p, at, i.read[y][x], i.trusted[y][x] != Tile::Unknown);
            if i.busy[y][x] {
                v.outline(p, at, cell, 2.0, BUSY);
            }
        }
    }
}

/// **Expects**: the board after the last move, what changed, and the next move.
fn expects(p: &mut Painter<'_>, v: &View, plan: &Plan) {
    let cell = SIDE / SIZE as f32;
    for y in 0..SIZE {
        for x in 0..SIZE {
            let at = [LEFTS[2] + x as f32 * cell, TOP + y as f32 * cell];
            if plan.after[y][x] != plan.before[y][x] {
                v.rect(p, at, [cell, cell], Color::rgba(1.0, 0.9, 0.4, 0.12));
            }

            v.tile(p, at, plan.after[y][x], true);
        }
    }

    for left in &LEFTS[..2] {
        v.swap(p, *left, plan.swap, MOVE);
    }

    if let Some((next, _)) = plan.next {
        v.swap(p, LEFTS[2], next, NEXT);
    }
}

/// The moves weighed for the last move made, best first: the one it made on top.
fn candidates(p: &mut Painter<'_>, v: &View, plan: Option<&Plan>) {
    let top = TOP + SIDE + 40.0;

    v.text(
        p,
        [LEFTS[0], top],
        "Moves weighed",
        22.0,
        Color::WHITE,
        Align::Left,
    );

    let cols = [0.0, 190.0, 270.0, 370.0, 460.0, 550.0].map(|x| LEFTS[0] + x);
    let heads = ["swap", "now", "later", "leaves", "worth", "lowest row"];
    for (x, h) in cols.iter().zip(heads) {
        v.text(p, [*x, top + 34.0], h, 14.0, MUTED, Align::Left);
    }

    let weighed = plan.map_or(&[][..], |pl| &pl.candidates);
    for (row, &(swap, value)) in weighed.iter().enumerate() {
        let y = top + 58.0 + row as f32 * 22.0;

        let color = if row == 0 { MOVE } else { MUTED };

        let Value {
            gain,
            total,
            left,
            depth,
        } = value;

        let cells = [
            swap_text(swap),
            gain.to_string(),
            format!("{:.1}", total - gain as f32),
            left.map_or("-".into(), |n| format!("{n} moves")),
            format!("{total:.1}"),
            depth.to_string(),
        ];

        for (x, c) in cols.iter().zip(cells) {
            v.text(p, [*x, y], &c, 16.0, color, Align::Left);
        }
    }
}

/// Moves, time and what each step of a round costs.
fn stats(p: &mut Painter<'_>, v: &View, i: &Insight) {
    let (left, top) = (LEFTS[2], TOP + SIDE + 40.0);

    v.text(p, [left, top], "Pace", 22.0, Color::WHITE, Align::Left);

    let rate = i.moves as f32 / i.elapsed.max(0.001);
    let [capture, read, solve, click] = i.timings;

    let lines = [
        format!(
            "{} moves in {:.0} s ({rate:.1} a second)",
            i.moves, i.elapsed
        ),
        format!("capture {capture:.0} ms   read {read:.1} ms"),
        format!("solve {solve:.1} ms   click {click:.0} ms"),
    ];

    for (k, l) in lines.iter().enumerate() {
        v.text(
            p,
            [left, top + 36.0 + k as f32 * 24.0],
            l,
            16.0,
            MUTED,
            Align::Left,
        );
    }

    let legend = [
        (MOVE, "the move it made"),
        (NEXT, "the move that one sets up"),
        (BUSY, "cells its moves still change"),
    ];

    for (k, (c, l)) in legend.iter().enumerate() {
        let y = top + 120.0 + k as f32 * 22.0;

        v.rect(p, [left, y + 3.0], [14.0, 14.0], *c);
        v.text(p, [left + 24.0, y], l, 15.0, MUTED, Align::Left);
    }
}

/// A swap as `(x, y) - (x, y)`.
fn swap_text((a, b): Swap) -> String {
    format!("({}, {}) - ({}, {})", a[0], a[1], b[0], b[1])
}

/// The colour the bot has in mind for a tile.
fn tile_color(t: Tile) -> Color {
    match t {
        Tile::Gem(WHITE) | Tile::Wild => Color::rgb(0.93, 0.94, 1.0),
        Tile::Gem(k) => hue(HUES[usize::from(k).min(HUES.len() - 1)]),
        Tile::Unknown => MUTED,
    }
}

/// A bright colour of `degrees` hue.
#[expect(
    clippy::many_single_char_names,
    reason = "h, x, r, g and b are the textbook names of the HSV conversion"
)]
fn hue(degrees: f32) -> Color {
    let h = degrees.rem_euclid(360.0) / 60.0;
    let x = 1.0 - (h % 2.0 - 1.0).abs();

    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "h is in 0..6; the sector is its whole part"
    )]
    let sector = h as u32;

    let (r, g, b) = match sector {
        0 => (1.0, x, 0.0),
        1 => (x, 1.0, 0.0),
        2 => (0.0, 1.0, x),
        3 => (0.0, x, 1.0),
        4 => (x, 0.0, 1.0),
        _ => (1.0, 0.0, x),
    };

    let lift = |c: f32| 0.15 + 0.8 * c;
    Color::rgb(lift(r), lift(g), lift(b))
}

/// Design space to window: one scale, centred.
struct View {
    /// Window pixels per design unit.
    scale: f32,
    /// Window position of the design origin.
    offset: [f32; 2],
}

impl View {
    /// Fits the design into a `w` × `h` window.
    fn fit(w: f32, h: f32) -> Self {
        let scale = (w / W).min(h / H);

        Self {
            scale,
            offset: [(w - W * scale) / 2.0, (h - H * scale) / 2.0],
        }
    }

    /// Window position of design point `d`.
    fn px(&self, d: [f32; 2]) -> [f32; 2] {
        [
            self.offset[0] + d[0] * self.scale,
            self.offset[1] + d[1] * self.scale,
        ]
    }

    /// Window length of design length `d`.
    fn len(&self, d: f32) -> f32 {
        d * self.scale
    }

    /// Text at design point `at`.
    fn text(&self, p: &mut Painter<'_>, at: [f32; 2], s: &str, size: f32, c: Color, align: Align) {
        p.draw_text(self.px(at), s)
            .size(self.len(size))
            .color(c)
            .align(align)
            .layer(TEXT);
    }

    /// A filled rectangle.
    fn rect(&self, p: &mut Painter<'_>, at: [f32; 2], size: [f32; 2], c: Color) {
        p.rect_ex(
            self.px(at),
            [self.len(size[0]), self.len(size[1])],
            c,
            SHAPES,
            BlendMode::Alpha,
        );
    }

    /// A square outline of side `side`.
    fn outline(&self, p: &mut Painter<'_>, at: [f32; 2], side: f32, width: f32, c: Color) {
        p.rect_outline_ex(
            self.px(at),
            [self.len(side); 2],
            self.len(width),
            c,
            MARKS,
            BlendMode::Alpha,
        );
    }

    /// A board's backing and frame at `at`.
    fn frame(&self, p: &mut Painter<'_>, at: [f32; 2]) {
        self.rect(p, at, [SIDE, SIDE], Color::rgb(0.1, 0.09, 0.16));

        p.rect_outline_ex(
            self.px([at[0] - 2.0, at[1] - 2.0]),
            [self.len(SIDE + 4.0); 2],
            self.len(1.0),
            Color::rgb(0.3, 0.28, 0.42),
            Layer(0),
            BlendMode::Alpha,
        );
    }

    /// One tile in the cell at `at`: a disc of its colour, hollow unless `solid`.
    fn tile(&self, p: &mut Painter<'_>, at: [f32; 2], t: Tile, solid: bool) {
        let cell = SIDE / SIZE as f32;
        let c = self.px([at[0] + cell / 2.0, at[1] + cell / 2.0]);
        let r = self.len(cell * 0.36);

        if t == Tile::Unknown {
            let mid = [at[0] + cell / 2.0, at[1] + cell * 0.22];
            self.text(p, mid, "?", 22.0, MUTED.with_alpha(0.6), Align::Center);
        } else {
            let width = if solid { 0.0 } else { self.len(2.5) };
            p.circle_ex(c, r, width, tile_color(t), SHAPES);

            if t == Tile::Wild {
                let mid = [at[0] + cell / 2.0, at[1] + cell * 0.25];
                self.text(p, mid, "W", 18.0, Color::rgb(0.1, 0.1, 0.2), Align::Center);
            }
        }
    }

    /// The swap `(a, b)` on the board whose left edge is at `left`: rings on both cells
    /// and a bar between them.
    fn swap(&self, p: &mut Painter<'_>, left: f32, (a, b): Swap, c: Color) {
        let cell = SIDE / SIZE as f32;

        let mid = |q: [usize; 2]| {
            self.px([
                left + (q[0] as f32 + 0.5) * cell,
                TOP + (q[1] as f32 + 0.5) * cell,
            ])
        };

        let (pa, pb) = (mid(a), mid(b));

        for q in [pa, pb] {
            p.circle_ex(q, self.len(cell * 0.47), self.len(3.0), c, MARKS);
        }

        p.line_ex(pa, pb, self.len(4.0), c, MARKS, BlendMode::Alpha);
    }
}
