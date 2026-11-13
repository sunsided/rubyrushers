//! The board as the player sees it: the resolved [`Step`]s played back as animations.
//!
//! The logic is always ahead; this view catches up step by step and is "busy" until it
//! has. Positions are in cells (fractional while moving).

use std::collections::{BTreeMap, VecDeque};

use crate::logic::{Board, Clear, Gem, Pos, SIZE, Step};

use super::easing;

/// Seconds a swap takes.
const SWAP_SECS: f32 = 0.16;
/// Seconds cleared gems take to vanish.
const CLEAR_SECS: f32 = 0.24;
/// Falling acceleration in cells per second squared.
const GRAVITY: f32 = 70.0;

/// Something the rest of the game reacts to (sound, particles, score popups).
#[derive(Clone, Debug)]
pub enum Cue {
    /// A swap started.
    Swapped,
    /// A swap that lined nothing up bounced back.
    Rejected,
    /// Gems are being cleared.
    Cleared(Clear),
    /// Falling gems landed.
    Landed,
    /// The board was reshuffled.
    Reshuffled,
}

/// A gem on screen.
#[derive(Clone, Copy, Debug)]
pub struct Sprite {
    /// The gem.
    pub gem: Gem,
    /// Position in cells.
    pub pos: [f32; 2],
    /// Size factor (1: fills its cell).
    pub scale: f32,
    /// Opacity.
    pub alpha: f32,
    /// Falling speed in cells per second.
    vy: f32,
    /// The row it falls to, while falling.
    target_y: Option<f32>,
}

impl Sprite {
    /// A resting gem at cell `p`.
    fn at(gem: Gem, p: Pos) -> Self {
        Self {
            gem,
            pos: [p.x as f32, p.y as f32],
            scale: 1.0,
            alpha: 1.0,
            vy: 0.0,
            target_y: None,
        }
    }
}

/// The animation running now.
#[derive(Clone, Debug)]
enum Anim {
    /// Two gems trading places (and back if `!ok`).
    Swap {
        /// Start cell and id of the first gem.
        a: (Pos, u32),
        /// Start cell and id of the second gem.
        b: (Pos, u32),
        /// Whether they stay swapped.
        ok: bool,
        /// Seconds since the start.
        t: f32,
    },
    /// Gems shrinking away.
    Clear {
        /// The gems on their way out.
        dying: Vec<Sprite>,
        /// Special gems to show once the others are gone.
        created: Vec<(Gem, Pos)>,
        /// Seconds since the start.
        t: f32,
    },
    /// Gems falling to their targets.
    Fall,
}

/// The animated board.
#[derive(Clone, Debug)]
pub struct BoardView {
    /// Gems on screen by id.
    sprites: BTreeMap<u32, Sprite>,
    /// Steps not yet started.
    queue: VecDeque<Step>,
    /// The step being animated.
    anim: Option<Anim>,
}

impl BoardView {
    /// A view showing `board` at rest.
    pub fn new(board: &Board) -> Self {
        let sprites = Pos::all()
            .filter_map(|p| board.get(p).map(|g| (g.id, Sprite::at(g, p))))
            .collect();
        Self {
            sprites,
            queue: VecDeque::new(),
            anim: None,
        }
    }

    /// Queues the steps of a move.
    pub fn push(&mut self, steps: Vec<Step>) {
        self.queue.extend(steps);
    }

    /// Whether animations are running or queued.
    pub fn busy(&self) -> bool {
        self.anim.is_some() || !self.queue.is_empty()
    }

    /// The gems to draw, the vanishing ones included.
    pub fn sprites(&self) -> impl Iterator<Item = &Sprite> {
        let dying = match &self.anim {
            Some(Anim::Clear { dying, .. }) => dying.as_slice(),
            _ => &[],
        };
        self.sprites.values().chain(dying)
    }

    /// Advances the animations by `dt` seconds.
    pub fn update(&mut self, dt: f32) -> Vec<Cue> {
        let mut cues = Vec::new();
        for s in self.sprites.values_mut() {
            s.scale += (1.0 - s.scale) * (dt * 14.0).min(1.0);
        }
        // Each finished animation starts the next queued step in the same frame.
        loop {
            if self.anim.is_none() {
                let Some(step) = self.queue.pop_front() else {
                    break;
                };
                self.start(step, &mut cues);
            }
            if !self.advance(dt, &mut cues) {
                break;
            }
        }
        cues
    }

    /// Starts animating `step`.
    fn start(&mut self, step: Step, cues: &mut Vec<Cue>) {
        self.anim = Some(match step {
            Step::Swap { a, b, ok } => {
                cues.push(Cue::Swapped);
                Anim::Swap { a, b, ok, t: 0.0 }
            }
            Step::Clear(clear) => {
                let dying = clear
                    .cleared
                    .iter()
                    .filter_map(|(g, _)| self.sprites.remove(&g.id))
                    .collect();
                let created = clear.created.clone();
                cues.push(Cue::Cleared(clear));
                Anim::Clear {
                    dying,
                    created,
                    t: 0.0,
                }
            }
            Step::Settle { falls, spawns } => {
                for (id, to) in falls {
                    if let Some(s) = self.sprites.get_mut(&id) {
                        s.target_y = Some(to.y as f32);
                    }
                }
                for (gem, to, from_row) in spawns {
                    let mut s = Sprite::at(gem, to);
                    s.pos[1] = from_row as f32;
                    s.target_y = Some(to.y as f32);
                    self.sprites.insert(gem.id, s);
                }
                Anim::Fall
            }
            Step::Reshuffle { gems } => {
                cues.push(Cue::Reshuffled);
                self.sprites.clear();
                for (gem, to) in gems {
                    let mut s = Sprite::at(gem, to);
                    s.pos[1] = (to.y - SIZE - 1) as f32 - (to.x as f32) * 0.3;
                    s.target_y = Some(to.y as f32);
                    self.sprites.insert(gem.id, s);
                }
                Anim::Fall
            }
        });
    }

    /// Runs the current animation; true when it finished this frame.
    fn advance(&mut self, dt: f32, cues: &mut Vec<Cue>) -> bool {
        let Some(anim) = &mut self.anim else {
            return false;
        };
        let done = match anim {
            Anim::Swap { a, b, ok, t } => {
                *t += dt;
                let total = if *ok { SWAP_SECS } else { SWAP_SECS * 2.0 };
                let k = (*t / total).min(1.0);
                // A failed swap goes halfway, then comes back.
                let f = if *ok {
                    easing::in_out_cubic(k)
                } else {
                    0.5 * (1.0 - (2.0 * k - 1.0).abs())
                };
                let (pa, pb) = (a.0, b.0);
                for ((from, id), to) in [(*a, pb), (*b, pa)] {
                    if let Some(s) = self.sprites.get_mut(&id) {
                        s.pos = lerp_cells(from, to, f);
                    }
                }
                if k >= 1.0 && !*ok {
                    cues.push(Cue::Rejected);
                }
                k >= 1.0
            }
            Anim::Clear { dying, created, t } => {
                *t += dt;
                let k = (*t / CLEAR_SECS).min(1.0);
                for s in dying.iter_mut() {
                    s.scale = 1.0 + 0.35 * (k * 3.0).min(1.0) - 1.35 * easing::out_cubic(k);
                    s.alpha = 1.0 - k * k;
                }
                if k >= 1.0 {
                    for &(gem, p) in created.iter() {
                        let mut s = Sprite::at(gem, p);
                        s.scale = 0.0;
                        self.sprites.insert(gem.id, s);
                    }
                }
                k >= 1.0
            }
            Anim::Fall => {
                let mut falling = false;
                for s in self.sprites.values_mut() {
                    if let Some(target) = s.target_y {
                        s.vy += GRAVITY * dt;
                        s.pos[1] += s.vy * dt;
                        if s.pos[1] >= target {
                            s.pos[1] = target;
                            s.vy = 0.0;
                            s.target_y = None;
                        } else {
                            falling = true;
                        }
                    }
                }
                if !falling {
                    cues.push(Cue::Landed);
                }
                !falling
            }
        };
        if done {
            self.anim = None;
        }
        done
    }
}

/// The point `f` of the way from cell `a` to cell `b`.
fn lerp_cells(a: Pos, b: Pos, f: f32) -> [f32; 2] {
    [
        a.x as f32 + (b.x - a.x) as f32 * f,
        a.y as f32 + (b.y - a.y) as f32 * f,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::{Mode, Session};

    #[test]
    fn playback_ends_where_the_logic_is() {
        let mut session = Session::new(Mode::Endless, 5);
        let mut view = BoardView::new(session.board());
        for _ in 0..20 {
            let Some((a, b)) = session.hint() else { break };
            view.push(session.swap(a, b));
            let mut frames = 0;
            while view.busy() {
                view.update(1.0 / 60.0);
                frames += 1;
                assert!(frames < 60 * 30, "animation never ends");
            }
            for p in Pos::all() {
                let gem = session.board().get(p).unwrap();
                let s = view.sprites.get(&gem.id).expect("every gem has a sprite");
                assert_eq!(s.pos, [p.x as f32, p.y as f32]);
            }
            assert_eq!(view.sprites.len() as i32, SIZE * SIZE);
        }
    }
}
