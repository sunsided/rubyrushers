//! Short-lived effects: sparks, blast rings and beams, score popups, screen shake.
//! Positions are in design units (see [`Layout`]).

use game_toolkit::prelude::*;

use super::gem_art::BASE;
use super::layout::{BOARD, BOARD_SIZE, Layout};
use crate::logic::{Clear, Pos, Power};

/// Layer of the sparks and blasts, above the gems.
const LAYER: Layer = Layer(4);
/// Most particles alive at once.
const MAX_PARTICLES: usize = 3000;

/// What a particle looks like.
#[derive(Clone, Copy, Debug)]
enum Shape {
    /// A spark that flies and falls.
    Dot,
    /// An expanding ring.
    Ring,
    /// A beam from its position to this point.
    Beam([f32; 2]),
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
    /// Radius (dot, ring at the end) or width (beam).
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
}

/// All running effects.
#[derive(Clone, Debug, Default)]
pub struct Fx {
    /// Sparks, rings and beams.
    particles: Vec<Particle>,
    /// Score texts.
    popups: Vec<Popup>,
    /// Screen shake strength in design units, decaying.
    shake: f32,
}

/// The design-space centre of cell `p`.
fn center(p: Pos) -> [f32; 2] {
    Layout::cell_center([p.x as f32, p.y as f32])
}

/// The display colour of gem colour `c`.
pub fn gem_color(c: u8) -> Color {
    let [r, g, b] = BASE[usize::from(c).min(BASE.len() - 1)];
    Color::rgb(r, g, b)
}

impl Fx {
    /// Removes every effect.
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// Effects for gems being cleared: sparks per gem, blasts, a score popup.
    pub fn cleared(&mut self, clear: &Clear, rng: &mut Rng) {
        for (gem, p) in &clear.cleared {
            let color = if gem.power == Power::Wild {
                Color::WHITE
            } else {
                gem_color(gem.color)
            };
            let n = if gem.power == Power::None { 7 } else { 18 };
            self.sparks(center(*p), color, n, rng);
        }
        for (power, at, reach) in &clear.blasts {
            let c = center(*at);
            match power {
                Power::Bomb => {
                    self.ring(c, Color::rgb(1.0, 0.7, 0.3), 120.0, 0.45);
                    self.shake = self.shake.max(7.0);
                }
                Power::Cross => {
                    let x0 = BOARD[0];
                    let y0 = BOARD[1];
                    self.beam([x0, c[1]], [x0 + BOARD_SIZE, c[1]], 22.0);
                    self.beam([c[0], y0], [c[0], y0 + BOARD_SIZE], 22.0);
                    self.shake = self.shake.max(9.0);
                }
                Power::Wild => {
                    for p in reach {
                        self.beam(c, center(*p), 6.0);
                    }
                    self.ring(c, Color::WHITE, 160.0, 0.6);
                    self.shake = self.shake.max(12.0);
                }
                Power::None => {}
            }
        }
        if clear.time_gems > 0 {
            self.popup(
                format!(
                    "+{} s",
                    clear.time_gems as f32 * crate::logic::mode::TIME_GEM_SECS
                ),
                [BOARD[0] + BOARD_SIZE / 2.0, BOARD[1] + BOARD_SIZE * 0.35],
                40.0,
                Color::rgb(0.5, 1.0, 0.9),
            );
        }
        let n = clear.cleared.len().max(1) as f32;
        let mid = clear.cleared.iter().fold([0.0, 0.0], |acc, (_, p)| {
            let c = center(*p);
            [acc[0] + c[0] / n, acc[1] + c[1] / n]
        });
        self.popup(
            format!("{}", clear.points),
            mid,
            24.0 + 4.0 * clear.cascade.min(6) as f32,
            Color::WHITE,
        );
        if clear.cascade >= 2 {
            self.popup(
                format!("Cascade x{}", clear.cascade),
                [BOARD[0] + BOARD_SIZE / 2.0, BOARD[1] + BOARD_SIZE / 2.0],
                36.0 + 6.0 * clear.cascade.min(6) as f32,
                Color::rgb(1.0, 0.85, 0.3),
            );
        }
    }

    /// A big centred message.
    pub fn banner(&mut self, text: &str) {
        self.popup(
            text.to_owned(),
            [BOARD[0] + BOARD_SIZE / 2.0, BOARD[1] + BOARD_SIZE / 2.0],
            56.0,
            Color::rgb(1.0, 0.95, 0.8),
        );
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
        });
    }

    /// Advances everything by `dt` seconds.
    pub fn update(&mut self, dt: f32) {
        for p in &mut self.particles {
            p.age += dt;
            if matches!(p.shape, Shape::Dot) {
                p.vel[1] += 900.0 * dt;
                p.pos[0] += p.vel[0] * dt;
                p.pos[1] += p.vel[1] * dt;
            }
        }
        self.particles.retain(|p| p.age < p.ttl);
        for p in &mut self.popups {
            p.age += dt;
            p.pos[1] -= 40.0 * dt;
        }
        self.popups.retain(|p| p.age < 1.1);
        self.shake = (self.shake - dt * 40.0).max(0.0);
    }

    /// The screen shake offset for this frame.
    pub fn shake(&self, rng: &mut Rng) -> [f32; 2] {
        if self.shake <= 0.0 {
            return [0.0; 2];
        }
        [
            (rng.next_f32() * 2.0 - 1.0) * self.shake,
            (rng.next_f32() * 2.0 - 1.0) * self.shake,
        ]
    }

    /// Draws everything.
    pub fn draw(&self, p: &mut Painter<'_>, layout: &Layout) {
        for q in &self.particles {
            let k = q.age / q.ttl;
            let fade = 1.0 - k;
            match q.shape {
                Shape::Dot => p.circle_ex(
                    layout.px(q.pos),
                    layout.len(q.size * (1.0 - 0.5 * k)),
                    0.0,
                    q.color.with_alpha(fade),
                    LAYER,
                ),
                Shape::Ring => p.circle_ex(
                    layout.px(q.pos),
                    layout.len(q.size * (0.2 + 0.8 * k)),
                    layout.len(6.0 * fade + 1.0),
                    q.color.with_alpha(fade),
                    LAYER,
                ),
                Shape::Beam(to) => p.line_ex(
                    layout.px(q.pos),
                    layout.px(to),
                    layout.len(q.size * fade),
                    q.color.with_alpha(fade * 0.8),
                    LAYER,
                    BlendMode::Additive,
                ),
            }
        }
        for q in &self.popups {
            let alpha = (1.0 - (q.age - 0.6).max(0.0) / 0.5).clamp(0.0, 1.0);
            let pop = 0.7 + 0.3 * super::easing::out_back((q.age / 0.2).min(1.0));
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
                .layer(Layer(5));
            p.draw_text([pos[0], pos[1] - size / 2.0], &q.text)
                .size(size)
                .align(Align::Center)
                .color(q.color.with_alpha(alpha))
                .layer(Layer(5));
        }
    }
}
