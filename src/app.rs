//! The game-toolkit glue: screens, input, and turning the logic's events into effects
//! and sound.

use std::path::PathBuf;
use std::sync::OnceLock;

use game_toolkit::prelude::*;

use crate::logic::{Event, Mode, Pos, Session, TimeRule};
use crate::render::scene::{self, Marks};
use crate::render::showcase;
use crate::render::{Effects, Fx, Layout, gem_art, hud};
use crate::scores::{self, Bests};
use crate::sound::{Sfx, Sound};

/// Seconds without input before a move is suggested.
const HINT_AFTER: f32 = 6.0;
/// Drag distance, in cells, that counts as a swipe.
const SWIPE_CELLS: f32 = 0.4;

/// Start-up options from the command line.
#[derive(Clone, Debug, Default)]
pub struct Options {
    /// Skip the title and start this mode.
    pub mode: Option<Mode>,
    /// Seed of the first game (else the clock's).
    pub seed: Option<u64>,
    /// Save a screenshot to this path after this many seconds, then quit.
    pub screenshot: Option<(PathBuf, f32)>,
    /// Start with calm effects.
    pub calm: bool,
    /// How much time gems add.
    pub time_rule: TimeRule,
}

/// The options set before [`run`].
static OPTIONS: OnceLock<Options> = OnceLock::new();

/// Sets the start-up options; call before [`run`].
pub fn set_options(options: Options) {
    let _ = OPTIONS.set(options);
}

/// The window and run-loop settings.
pub fn config() -> AppConfig {
    AppConfig {
        title: "RubyRushers".into(),
        width: 1120,
        height: 840,
        asset_root: asset_root!(),
        canvas_id: Some("game".into()),
        touch_emulates_mouse: true,
        msaa_samples: 4,
        ..Default::default()
    }
}

/// Which screen is up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Screen {
    /// Mode selection over a resting board.
    Title,
    /// A game in progress.
    Playing,
    /// The final score.
    Over {
        /// Whether the score beat the previous best.
        new_best: bool,
    },
}

/// The game.
pub struct App {
    /// The screen that is up.
    screen: Screen,
    /// The game being played (or shown behind the title).
    session: Session,
    /// The gem atlas.
    atlas: TextureId,
    /// Sparks, popups, shake, vanishing gems.
    fx: Fx,
    /// How much moves and sparkles.
    effects: Effects,
    /// Sound effects.
    sound: Sound,
    /// The gem the player picked.
    selected: Option<Pos>,
    /// Where the current press started: window position and cell.
    press: Option<([f32; 2], Pos)>,
    /// Seconds without input while the board rests.
    idle: f32,
    /// Seconds since start.
    clock: f32,
    /// Score shown in the panel, counting up towards `shown_target`.
    shown_score: f32,
    /// Score so far, as the events arrived.
    shown_target: u64,
    /// Best score per mode (title button order).
    bests: Bests,
    /// The seed of the next game.
    seed: u64,
    /// How much time gems add.
    time_rule: TimeRule,
    /// The screenshot asked for on the command line, until taken.
    screenshot: Option<(PathBuf, f32)>,
    /// Quit at the next update (the screenshot has been taken).
    quit_next: bool,
}

impl App {
    /// Starts a new game of `mode`.
    fn start(&mut self, mode: Mode) {
        // `--seed` deals exactly the first game; later ones step on from it.
        self.session = Session::new(mode, self.seed).with_time_rule(self.time_rule);
        self.seed = self.seed.wrapping_add(0x9E37_79B9);
        self.fx.clear();
        self.fx.banner("Go!");
        self.selected = None;
        self.press = None;
        self.idle = 0.0;
        self.shown_score = 0.0;
        self.shown_target = 0;
        self.screen = Screen::Playing;
    }

    /// Index of the current mode in the title's button order.
    fn mode_index(&self) -> usize {
        scores::index(self.session.mode())
    }

    /// Tries to swap the gems at `a` and `b`; if they cannot move yet, `b` becomes the
    /// picked gem instead.
    fn swap(&mut self, a: Pos, b: Pos) {
        let swapped = self.session.swap(a, b);
        self.selected = (!swapped && self.session.can_move(b)).then_some(b);
        self.press = None;
        self.idle = 0.0;
    }

    /// Click-click and drag-to-swap on the board.
    fn board_input(&mut self, ctx: &Context, layout: &Layout) {
        let (mx, my) = ctx.input.mouse_pos();
        let mouse = [mx, my];
        if ctx.input.mouse_pressed(MouseButton::Left) {
            self.idle = 0.0;
            match layout.cell_at(mouse) {
                Some(c) if self.selected.is_some_and(|s| s.adjacent(c)) => {
                    let s = self.selected.unwrap_or(c);
                    self.swap(s, c);
                }
                Some(c) if self.session.can_move(c) => {
                    self.selected = (self.selected != Some(c)).then_some(c);
                    self.press = Some((mouse, c));
                }
                Some(_) => {}
                None => self.selected = None,
            }
        }
        // A flick can start and end within one frame, so measure before letting go.
        if let Some((start, c)) = self.press {
            let d = [mouse[0] - start[0], mouse[1] - start[1]];
            if d[0].abs().max(d[1].abs()) > SWIPE_CELLS * layout.cell() {
                let to = if d[0].abs() > d[1].abs() {
                    Pos::new(c.x + d[0].signum() as i32, c.y)
                } else {
                    Pos::new(c.x, c.y + d[1].signum() as i32)
                };
                if to.on_board() {
                    self.swap(c, to);
                }
                self.press = None;
            } else if !ctx.input.mouse_held(MouseButton::Left) {
                self.press = None;
            }
        }
    }

    /// Effects and sounds for what just happened on the board.
    fn react(&mut self, ctx: &mut Context, events: Vec<Event>) {
        for event in events {
            let sfx = match &event {
                Event::Swapped => Some(Sfx::Swap),
                Event::Rejected => Some(Sfx::Reject),
                Event::Landed => Some(Sfx::Land),
                Event::Reshuffled => {
                    self.fx.banner("Reshuffle");
                    None
                }
                Event::Tilted(_) => {
                    self.fx.banner("TILT!");
                    self.fx.jolt();
                    Some(Sfx::Blast)
                }
                Event::Over => {
                    let i = self.mode_index();
                    let new_best = self.session.score() > self.bests[i];
                    if new_best {
                        self.bests[i] = self.session.score();
                        scores::save(&self.bests);
                    }
                    self.screen = Screen::Over { new_best };
                    self.selected = None;
                    Some(Sfx::Over)
                }
                Event::Cleared(clear) => {
                    self.shown_target += clear.points;
                    self.fx.cleared(clear, &mut ctx.rng, self.effects);
                    if !clear.blasts.is_empty() {
                        self.sound.play(ctx.audio.as_mut(), Sfx::Blast);
                    }
                    if clear.time_gems > 0 {
                        self.sound.play(ctx.audio.as_mut(), Sfx::Time);
                    }
                    Some(Sfx::Match(clear.cascade))
                }
            };
            if let Some(sfx) = sfx {
                self.sound.play(ctx.audio.as_mut(), sfx);
            }
        }
    }

    /// F12 saves a screenshot; a command-line one is taken when due, then the app quits.
    fn screenshots(&mut self, ctx: &mut Context) {
        if self.quit_next {
            ctx.quit();
            return;
        }
        let due = self
            .screenshot
            .as_ref()
            .is_some_and(|(_, at)| self.clock >= *at);
        let path = if due {
            self.quit_next = true;
            self.screenshot.take().map(|(p, _)| p)
        } else if ctx.input.key_pressed(Key::F12) {
            Some(PathBuf::from(format!(
                "screenshots/rubyrushers-{}.png",
                ctx.time.frame
            )))
        } else {
            None
        };
        if let Some(p) = path {
            if let Some(dir) = p.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            log::info!("screenshot {}", p.display());
            ctx.gfx.request_screenshot(p);
        }
    }

    /// The layout for the current window.
    fn layout(ctx: &Context) -> Layout {
        let (w, h) = ctx.gfx.size();
        Layout::fit(w as f32, h as f32)
    }
}

impl Game for App {
    /// Draws the gem atlas and sets up the title screen (or the mode from the command
    /// line).
    fn init(ctx: &mut Context) -> Result<Self> {
        let options = OPTIONS.get().cloned().unwrap_or_default();
        let seed = options.seed.unwrap_or_else(clock_seed);
        let atlas = ctx.gfx.create_texture_rgba(
            gem_art::TILE * gem_art::TILES,
            gem_art::TILE,
            &gem_art::atlas(),
            Some("gems"),
        );
        let session = Session::new(Mode::Endless, seed);
        let mut app = Self {
            screen: Screen::Title,
            session,
            atlas,
            fx: Fx::default(),
            effects: if options.calm || prefers_reduced_motion() {
                Effects::Calm
            } else {
                Effects::Full
            },
            sound: Sound::new(),
            selected: None,
            press: None,
            idle: 0.0,
            clock: 0.0,
            shown_score: 0.0,
            shown_target: 0,
            bests: scores::load(),
            seed,
            time_rule: options.time_rule,
            screenshot: options.screenshot,
            quit_next: false,
        };
        if let Some(mode) = options.mode {
            app.start(mode);
        }
        Ok(app)
    }

    /// Input, the clock, and the animations.
    fn update(&mut self, ctx: &mut Context, dt: f32) {
        self.clock += dt;
        self.screenshots(ctx);
        let layout = Self::layout(ctx);
        if ctx.input.key_pressed(Key::F2) {
            self.effects = self.effects.toggled();
        }
        self.fx
            .update(dt, &mut ctx.rng, self.effects, self.session.pieces());
        let target = self.shown_target as f32;
        self.shown_score += (target - self.shown_score) * (dt * 8.0).min(1.0);
        if (target - self.shown_score).abs() < 1.0 {
            self.shown_score = target;
        }

        match self.screen {
            Screen::Title => {
                let (mx, my) = ctx.input.mouse_pos();
                let clicked = ctx
                    .input
                    .mouse_pressed(MouseButton::Left)
                    .then(|| hud::button_at(layout.design([mx, my])))
                    .flatten();
                let key = if ctx.input.key_pressed(Key::Digit1) {
                    Some(0)
                } else if ctx.input.key_pressed(Key::Digit2) {
                    Some(1)
                } else if ctx.input.key_pressed(Key::Digit3) {
                    Some(2)
                } else {
                    None
                };
                if let Some(i) = clicked.or(key) {
                    self.sound.play(ctx.audio.as_mut(), Sfx::Click);
                    self.start(hud::MODES[i]);
                } else if ctx.input.key_pressed(Key::Escape) {
                    ctx.quit();
                }
            }
            Screen::Playing => {
                if ctx.input.key_pressed(Key::Escape) {
                    self.screen = Screen::Title;
                    return;
                }
                self.board_input(ctx, &layout);
                if self.selected.is_some_and(|c| !self.session.can_move(c)) {
                    self.selected = None;
                }
                self.session.update(dt);
                let events = self.session.drain_events();
                self.react(ctx, events);
                if self.session.is_still() {
                    self.idle += dt;
                }
            }
            Screen::Over { .. } => {
                if ctx.input.mouse_pressed(MouseButton::Left)
                    || ctx.input.key_pressed(Key::Enter)
                    || ctx.input.key_pressed(Key::Escape)
                {
                    self.sound.play(ctx.audio.as_mut(), Sfx::Click);
                    self.screen = Screen::Title;
                }
            }
        }
    }

    /// Background, board, effects, then the screen's HUD.
    fn render(&mut self, ctx: &mut Context, frame: &mut Frame) {
        let (w, h) = ctx.gfx.size();
        let (w, h) = (w as f32, h as f32);
        let shake = self.fx.shake(&mut ctx.rng, self.effects);
        let base = Layout::fit(w, h);
        let layout = base.shifted(shake);
        let playing = self.screen == Screen::Playing;
        let marks = Marks {
            selected: if playing { self.selected } else { None },
            hint: (playing && self.idle > HINT_AFTER)
                .then(|| self.session.hint())
                .flatten(),
        };
        let mouse = {
            let (mx, my) = ctx.input.mouse_pos();
            base.design([mx, my])
        };

        let mut p = frame.painter(&mut ctx.gfx);
        scene::background(&mut p, w, h);
        // The title stands alone; the board only shows once a game is dealt.
        if self.screen != Screen::Title {
            if self.effects.full() {
                showcase::glows(&mut p, &base, self.clock, 0.1);
            }
            let (session, fx, effects) = (&self.session, &self.fx, self.effects);
            scene::board(
                &mut p, &layout, session, fx, self.atlas, self.clock, marks, effects,
            );
            self.fx.draw(&mut p, &layout, self.effects);
        }
        match self.screen {
            Screen::Title => {
                showcase::backdrop(&mut p, &base, self.atlas, self.clock, self.effects);
                hud::title(
                    &mut p,
                    &base,
                    &self.bests,
                    hud::button_at(mouse),
                    self.clock,
                    self.effects,
                );
            }
            Screen::Playing => {
                let best = self.bests[self.mode_index()];
                hud::panel(
                    &mut p,
                    &base,
                    &self.session,
                    self.shown_score as u64,
                    best,
                    self.clock,
                    self.effects,
                );
                hud::tilt_warning(&mut p, &base, &self.session, self.clock, self.effects);
            }
            Screen::Over { new_best } => {
                let best = self.bests[self.mode_index()];
                hud::panel(
                    &mut p,
                    &base,
                    &self.session,
                    self.shown_score as u64,
                    best,
                    self.clock,
                    self.effects,
                );
                hud::game_over(&mut p, &base, w, h, &self.session, best, new_best);
            }
        }
    }
}

/// Whether the browser asks for reduced motion (`prefers-reduced-motion: reduce`).
fn prefers_reduced_motion() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window()
            .and_then(|w| {
                w.match_media("(prefers-reduced-motion: reduce)")
                    .ok()
                    .flatten()
            })
            .is_some_and(|m| m.matches())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        false
    }
}

/// A seed from the wall clock, so every launch deals new boards.
fn clock_seed() -> u64 {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(1, |d| d.as_nanos() as u64)
    }
    // The web has no SystemTime; the page's clock in milliseconds will do.
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::js_sys::Date::now() as u64
    }
}
