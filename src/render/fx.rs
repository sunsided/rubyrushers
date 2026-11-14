//! Short-lived effects: vanishing and newborn gems, sparks, blast rings and beams,
//! glints, drifting motes, score popups, screen shake. Positions are in design units
//! (see [`Layout`]) unless they are gem cells.

use game_toolkit::prelude::*;

use super::easing;
use super::effects::Effects;
use super::gem_art::BASE;
use super::layout::{BOARD, BOARD_SIZE, DESIGN_H, DESIGN_W, Layout};
use crate::logic::mode::TIME_GEM_SECS;
use crate::logic::score;
use crate::logic::{Clear, Gem, Piece, Power};

/// Layer of the background motes, above the background gradient.
const BACK: Layer = Layer(0);
/// Layer of the sparks and blasts, above the gems.
const FRONT: Layer = Layer(4);
/// Layer of the popups.
const TEXT: Layer = Layer(5);
/// Most particles alive at once.
const MAX_PARTICLES: usize = 3000;
/// Seconds a cleared gem takes to vanish.
const VANISH_SECS: f32 = 0.25;
/// Seconds a new special gem takes to pop in.
const BORN_SECS: f32 = 0.3;

/// What a particle looks like.
#[derive(Clone, Copy, Debug)]
enum Shape {
    /// A spark that flies and falls.
    Dot,
    /// An expanding ring.
    Ring,
    /// A beam from its position to this point.
    Beam([f32; 2]),
    /// A four-pointed twinkle.
    Glint,
    /// A faint dot drifting behind the board.
    Mote,
}

/// One particle.
#[derive(Clone, Copy, Debug)]
struct Particle {
    /// Where.
    pos: [f32; 2],
    /// Speed in design units per second.
    vel: [f32; 2],
    /// Seconds lived.
    age: f32,
    /// Seconds it lives.
    ttl: f32,
    /// Colour.
    color: Color,
    /// Radius (dot, glint, ring at the end) or width (beam).
    size: f32,
    /// Look.
    shape: Shape,
}

/// A floating text.
#[derive(Clone, Debug)]
struct Popup {
    /// The text.
    text: String,
    /// Where (its centre).
    pos: [f32; 2],
    /// Seconds lived.
    age: f32,
    /// Text size in design units.
    size: f32,
    /// Colour.
    color: Color,
    /// A cascade count: a newer one replaces it.
    cascade: bool,
}

/// A cleared gem on its way out.
#[derive(Clone, Copy, Debug)]
pub struct Vanish {
    /// The gem.
    pub gem: Gem,
    /// Where it was drawn, in cells.
    pub pos: [f32; 2],
    /// Seconds since it was cleared.
    age: f32,
}

impl Vanish {
    /// Size factor: a short swell, then shrinking away.
    pub fn scale(&self) -> f32 {
        let k = (self.age / VANISH_SECS).min(1.0);
        1.0 + 0.3 * (k * 3.0).min(1.0) - 1.3 * easing::out_cubic(k)
    }

    /// Opacity.
    pub fn alpha(&self) -> f32 {
        let k = (self.age / VANISH_SECS).min(1.0);
        1.0 - k * k
    }

    /// Spin in radians (full effects).
    pub fn spin(&self) -> f32 {
        self.age * 9.0
    }
}

/// All running effects.
#[derive(Clone, Debug, Default)]
pub struct Fx {
    /// Sparks, rings, beams, glints, motes.
    particles: Vec<Particle>,
    /// Score texts.
    popups: Vec<Popup>,
    /// Cleared gems shrinking away.
    vanishing: Vec<Vanish>,
    /// New special gems popping in: id and seconds since.
    born: Vec<(u32, f32)>,
    /// Screen shake strength in design units, decaying.
    shake: f32,
}

/// The design-space centre of (fractional) cell `c`.
fn center(c: [f32; 2]) -> [f32; 2] {
    Layout::cell_center(c)
}

/// The display colour of gem colour `c`.
pub fn gem_color(c: u8) -> Color {
    let [r, g, b] = BASE[usize::from(c).min(BASE.len() - 1)];
    Color::rgb(r, g, b)
}

/// The middle of the board in the design space.
const BOARD_MID: [f32; 2] = [BOARD[0] + BOARD_SIZE / 2.0, BOARD[1] + BOARD_SIZE / 2.0];

impl Fx {
    /// Removes every effect.
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// Effects for gems being cleared.
    pub fn cleared(&mut self, clear: &Clear, rng: &mut Rng, effects: Effects) {
        for &(gem, pos) in &clear.cleared {
            self.vanishing.push(Vanish { gem, pos, age: 0.0 });
            let color = if gem.power == Power::Wild {
                Color::WHITE
            } else {
                gem_color(gem.color)
            };
            let n = match (effects, gem.power) {
                (Effects::Calm, _) => 2,
                (Effects::Full, Power::None) => 9,
                (Effects::Full, _) => 24,
            };
            self.sparks(center(pos), color, n, rng);
        }
        for (gem, _) in &clear.created {
            self.born.push((gem.id, 0.0));
        }
        for (power, at, reach) in &clear.blasts {
            let c = center([at.x as f32, at.y as f32]);
            match power {
                Power::Bomb => {
                    self.ring(c, Color::rgb(1.0, 0.7, 0.3), 120.0, 0.45);
                    self.shake = self.shake.max(7.0);
                }
                Power::Cross => {
                    self.beam([BOARD[0], c[1]], [BOARD[0] + BOARD_SIZE, c[1]], 22.0);
                    self.beam([c[0], BOARD[1]], [c[0], BOARD[1] + BOARD_SIZE], 22.0);
                    self.shake = self.shake.max(9.0);
                }
                Power::Wild => {
                    for p in reach {
                        self.beam(c, center([p.x as f32, p.y as f32]), 6.0);
                    }
                    self.ring(c, Color::WHITE, 160.0, 0.6);
                    self.shake = self.shake.max(12.0);
                }
                Power::None => {}
            }
        }
        if clear.time_gems > 0 {
            let secs = clear.time_gems as f32 * TIME_GEM_SECS;
            let at = [BOARD_MID[0], BOARD[1] + BOARD_SIZE * 0.35];
            self.popup(format!("+{secs} s"), at, 40.0, Color::rgb(0.5, 1.0, 0.9));
        }
        let n = clear.cleared.len().max(1) as f32;
        let mid = clear.cleared.iter().fold([0.0, 0.0], |acc, (_, p)| {
            let c = center(*p);
            [acc[0] + c[0] / n, acc[1] + c[1] / n]
        });
        let size = 24.0 + 4.0 * clear.cascade.min(6) as f32;
        self.popup(clear.points.to_string(), mid, size, Color::WHITE);
        if clear.cascade == 1 && clear.speed > 0 {
            let text = format!("Speed x{}", score::speed_factor(clear.speed));
            let at = [mid[0], mid[1] - 34.0];
            self.popup(text, at, 22.0, Color::rgb(1.0, 0.45, 0.5));
        }
        if clear.cascade >= 2 {
            let size = 36.0 + 6.0 * clear.cascade.min(6) as f32;
            let text = format!("Cascade x{}", clear.cascade);
            self.popups.retain(|p| !p.cascade);
            self.popup(text, BOARD_MID, size, Color::rgb(1.0, 0.85, 0.3));
            if let Some(last) = self.popups.last_mut() {
                last.cascade = true;
            }
        }
    }

    /// A hard shake, for gravity turning.
    pub fn jolt(&mut self) {
        self.shake = self.shake.max(14.0);
    }

    /// A big centred message.
    pub fn banner(&mut self, text: &str) {
        self.popup(text.to_owned(), BOARD_MID, 56.0, Color::rgb(1.0, 0.95, 0.8));
    }

    /// `n` sparks flying out of `at`.
    fn sparks(&mut self, at: [f32; 2], color: Color, n: usize, rng: &mut Rng) {
        for _ in 0..n {
            let a = rng.next_f32() * std::f32::consts::TAU;
            let speed = 80.0 + rng.next_f32() * 260.0;
            self.push(Particle {
                pos: at,
                vel: [a.cos() * speed, a.sin() * speed - 120.0],
                age: 0.0,
                ttl: 0.4 + rng.next_f32() * 0.5,
                color,
                size: 2.0 + rng.next_f32() * 3.5,
                shape: Shape::Dot,
            });
        }
    }

    /// An expanding ring at `at`.
    fn ring(&mut self, at: [f32; 2], color: Color, radius: f32, ttl: f32) {
        self.push(Particle {
            pos: at,
            vel: [0.0; 2],
            age: 0.0,
            ttl,
            color,
            size: radius,
            shape: Shape::Ring,
        });
    }

    /// A fading beam from `a` to `b`.
    fn beam(&mut self, a: [f32; 2], b: [f32; 2], width: f32) {
        self.push(Particle {
            pos: a,
            vel: [0.0; 2],
            age: 0.0,
            ttl: 0.4,
            color: Color::rgb(1.0, 0.95, 0.75),
            size: width,
            shape: Shape::Beam(b),
        });
    }

    /// Adds a particle unless there are too many.
    fn push(&mut self, p: Particle) {
        if self.particles.len() < MAX_PARTICLES {
            self.particles.push(p);
        }
    }

    /// A text floating up from `pos`.
    fn popup(&mut self, text: String, pos: [f32; 2], size: f32, color: Color) {
        self.popups.push(Popup {
            text,
            pos,
            age: 0.0,
            size,
            color,
            cascade: false,
        });
    }

    /// Advances everything by `dt` seconds; with full effects, resting gems glint now and
    /// then and motes drift behind the board.
    pub fn update(
        &mut self,
        dt: f32,
        rng: &mut Rng,
        effects: Effects,
        pieces: impl Iterator<Item = Piece>,
    ) {
        for p in &mut self.particles {
            p.age += dt;
            match p.shape {
                Shape::Dot => {
                    p.vel[1] += 900.0 * dt;
                    p.pos[0] += p.vel[0] * dt;
                    p.pos[1] += p.vel[1] * dt;
                }
                Shape::Mote => {
                    p.pos[0] += p.vel[0] * dt;
                    p.pos[1] += p.vel[1] * dt;
                }
                _ => {}
            }
        }
        self.particles.retain(|p| p.age < p.ttl);
        for p in &mut self.popups {
            p.age += dt;
            if effects.full() {
                p.pos[1] -= 40.0 * dt;
            }
        }
        self.popups.retain(|p| p.age < 1.1);
        for v in &mut self.vanishing {
            v.age += dt;
        }
        self.vanishing.retain(|v| v.age < VANISH_SECS);
        for b in &mut self.born {
            b.1 += dt;
        }
        self.born.retain(|b| b.1 < BORN_SECS);
        self.shake = (self.shake - dt * 40.0).max(0.0);

        if effects.full() {
            self.ambient(dt, rng, pieces);
        } else {
            self.particles
                .retain(|p| !matches!(p.shape, Shape::Glint | Shape::Mote));
        }
    }

    /// Glints on resting gems and motes behind the board.
    fn ambient(&mut self, dt: f32, rng: &mut Rng, pieces: impl Iterator<Item = Piece>) {
        // About three glints a second across the board.
        if rng.next_f32() < dt * 3.0 {
            let resting: Vec<Piece> = pieces.filter(|q| q.speed == 0.0).collect();
            if !resting.is_empty() {
                let q = resting[rng.range_usize(0..resting.len())];
                let c = center(q.pos);
                let off = [rng.next_f32() * 24.0 - 18.0, rng.next_f32() * 24.0 - 20.0];
                self.push(Particle {
                    pos: [c[0] + off[0], c[1] + off[1]],
                    vel: [0.0; 2],
                    age: 0.0,
                    ttl: 0.55,
                    color: Color::WHITE,
                    size: 9.0 + rng.next_f32() * 6.0,
                    shape: Shape::Glint,
                });
            }
        }
        if rng.next_f32() < dt * 6.0 {
            self.push(Particle {
                pos: [rng.next_f32() * DESIGN_W, DESIGN_H + 10.0],
                vel: [rng.next_f32() * 16.0 - 8.0, -(15.0 + rng.next_f32() * 25.0)],
                age: 0.0,
                ttl: 30.0,
                color: Color::rgb(0.7, 0.6, 1.0),
                size: 1.0 + rng.next_f32() * 2.0,
                shape: Shape::Mote,
            });
        }
    }

    /// The screen shake offset for this frame (none when calm).
    pub fn shake(&self, rng: &mut Rng, effects: Effects) -> [f32; 2] {
        if self.shake <= 0.0 || !effects.full() {
            return [0.0; 2];
        }
        [
            (rng.next_f32() * 2.0 - 1.0) * self.shake,
            (rng.next_f32() * 2.0 - 1.0) * self.shake,
        ]
    }

    /// Size factor of the gem with `id`: below 1 while a new special gem pops in.
    pub fn born_scale(&self, id: u32) -> f32 {
        self.born
            .iter()
            .find(|b| b.0 == id)
            .map_or(1.0, |b| easing::out_back(b.1 / BORN_SECS))
    }

    /// Cleared gems still shrinking away.
    pub fn vanishing(&self) -> &[Vanish] {
        &self.vanishing
    }

    /// Draws particles and popups.
    pub fn draw(&self, p: &mut Painter<'_>, layout: &Layout, effects: Effects) {
        for q in &self.particles {
            let k = q.age / q.ttl;
            let fade = 1.0 - k;
            match q.shape {
                Shape::Dot => p.circle_ex(
                    layout.px(q.pos),
                    layout.len(q.size * (1.0 - 0.5 * k)),
                    0.0,
                    q.color.with_alpha(fade),
                    FRONT,
                ),
                Shape::Ring => p.circle_ex(
                    layout.px(q.pos),
                    layout.len(q.size * (0.2 + 0.8 * k)),
                    layout.len(6.0 * fade + 1.0),
                    q.color.with_alpha(fade),
                    FRONT,
                ),
                Shape::Beam(to) => p.line_ex(
                    layout.px(q.pos),
                    layout.px(to),
                    layout.len(q.size * fade),
                    q.color.with_alpha(fade * 0.8),
                    FRONT,
                    BlendMode::Additive,
                ),
                Shape::Glint => {
                    // Grows and shrinks again: two crossed thin rays.
                    let s = layout.len(q.size * (1.0 - (2.0 * k - 1.0).abs()));
                    let c = layout.px(q.pos);
                    let w = layout.len(1.6);
                    let color = q.color.with_alpha(0.9);
                    let add = BlendMode::Additive;
                    p.line_ex([c[0] - s, c[1]], [c[0] + s, c[1]], w, color, FRONT, add);
                    p.line_ex([c[0], c[1] - s], [c[0], c[1] + s], w, color, FRONT, add);
                }
                Shape::Mote => p.circle_ex(
                    layout.px(q.pos),
                    layout.len(q.size),
                    0.0,
                    q.color.with_alpha(0.25),
                    BACK,
                ),
            }
        }
        for q in &self.popups {
            let alpha = (1.0 - (q.age - 0.6).max(0.0) / 0.5).clamp(0.0, 1.0);
            let pop = if effects.full() {
                0.7 + 0.3 * easing::out_back((q.age / 0.2).min(1.0))
            } else {
                1.0
            };
            let size = layout.len(q.size * pop);
            let pos = layout.px(q.pos);
            let shadow = [
                pos[0] + layout.len(2.0),
                pos[1] - size / 2.0 + layout.len(2.0),
            ];
            p.draw_text(shadow, &q.text)
                .size(size)
                .align(Align::Center)
                .color(Color::BLACK.with_alpha(alpha * 0.6))
                .layer(TEXT);
            p.draw_text([pos[0], pos[1] - size / 2.0], &q.text)
                .size(size)
                .align(Align::Center)
                .color(q.color.with_alpha(alpha))
                .layer(TEXT);
        }
    }
}
