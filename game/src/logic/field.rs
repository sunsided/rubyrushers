//! The board in motion: every gem's cell plus how it is moving, gravity, and the holes
//! left by cleared gems.
//!
//! A gem always belongs to one cell; moving gems are drawn offset from it. Only settled
//! gems (no swap, no fall) match or can be swapped, so the player may keep playing on the
//! parts of the board that rest while others still fall.

use super::board::{Board, SIZE};
use super::gem::{COLORS, Gem};
use super::gravity::Gravity;
use super::motion::{GRAVITY, Motion, Swap};
use super::pos::Pos;
use super::power::Power;
use super::rng::Rng;

/// Seconds a cleared cell stays blocked before the gems above fall into it.
pub const HOLD_SECS: f32 = 0.22;

/// Cells on the board.
const CELLS: usize = (SIZE * SIZE) as usize;

/// A gem as drawn: the gem, where (fractional cells) and how fast it falls.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Piece {
    /// The gem.
    pub gem: Gem,
    /// Position in cells (column, row), offset included.
    pub pos: [f32; 2],
    /// Falling speed in cells per second (0 at rest).
    pub speed: f32,
    /// Whether it is in a swap (and may overlap its partner).
    pub swapping: bool,
}

/// The gems, their motion and the blocked cells.
#[derive(Clone, Debug)]
pub struct Field {
    /// Which gem belongs to which cell.
    board: Board,
    /// Per cell: the motion of its gem.
    motion: [Motion; CELLS],
    /// Per cell: seconds it stays blocked after a clear (0: open).
    hold: [f32; CELLS],
    /// Per cell: cascade depth of the clear that emptied it; gems falling past it carry it.
    hole_chain: [u32; CELLS],
    /// Which way gems fall.
    gravity: Gravity,
}

/// The index of an on-board cell.
fn idx(p: Pos) -> usize {
    usize::try_from(p.y * SIZE + p.x).expect("an on-board cell")
}

impl Field {
    /// `board` at rest.
    #[must_use]
    pub fn new(board: Board) -> Self {
        Self {
            board,
            motion: [Motion::default(); CELLS],
            hold: [0.0; CELLS],
            hole_chain: [0; CELLS],
            gravity: Gravity::Down,
        }
    }

    /// Which way gems fall.
    #[must_use]
    pub fn gravity(&self) -> Gravity {
        self.gravity
    }

    /// Turns gravity to `g`; only while nothing moves (returns whether it turned).
    pub fn set_gravity(&mut self, g: Gravity) -> bool {
        let still = self.is_still();
        if still {
            self.gravity = g;
        }

        still
    }

    /// The gems by cell, moving ones included.
    #[must_use]
    pub fn board(&self) -> &Board {
        &self.board
    }

    /// The motion of the gem at `p`.
    #[must_use]
    pub fn motion(&self, p: Pos) -> &Motion {
        &self.motion[idx(p)]
    }

    /// The motion of the gem at `p`, to change.
    pub fn motion_mut(&mut self, p: Pos) -> &mut Motion {
        &mut self.motion[idx(p)]
    }

    /// Whether `p` holds a gem at rest in an open cell.
    #[must_use]
    pub fn is_settled(&self, p: Pos) -> bool {
        self.board.get(p).is_some() && self.motion(p).settled() && self.hold[idx(p)] <= 0.0
    }

    /// Whether nothing moves: every cell has a settled gem.
    #[must_use]
    pub fn is_still(&self) -> bool {
        Pos::all().all(|p| self.is_settled(p))
    }

    /// Only the settled gems, for matching.
    #[must_use]
    pub fn settled_board(&self) -> Board {
        let mut b = self.board.clone();

        for p in Pos::all() {
            if !self.is_settled(p) {
                b.set(p, None);
            }
        }

        b
    }

    /// Every gem as drawn.
    pub fn pieces(&self) -> impl Iterator<Item = Piece> + '_ {
        Pos::all().filter_map(|p| {
            let gem = self.board.get(p)?;
            let m = self.motion(p);
            Some(Piece {
                gem,
                pos: self.drawn_at(p),
                speed: m.vy,
                swapping: m.swap.is_some(),
            })
        })
    }

    /// Where the gem at `p` is drawn, in cells.
    #[must_use]
    pub fn drawn_at(&self, p: Pos) -> [f32; 2] {
        let o = self.motion(p).offset(self.gravity.dir());
        [p.x as f32 + o[0], p.y as f32 + o[1]]
    }

    /// Exchanges two gems and animates them into their new cells.
    pub fn start_swap(&mut self, a: Pos, b: Pos) {
        self.board.swap(a, b);
        self.motion.swap(idx(a), idx(b));

        for (at, from) in [(a, b), (b, a)] {
            let m = self.motion_mut(at);
            m.swap = Some(Swap {
                dir: [(from.x - at.x) as f32, (from.y - at.y) as f32],
                t: 0.0,
                bounce: false,
            });
            m.moved = true;
            m.chain = 1;
        }
    }

    /// Animates two gems trying to swap and coming back.
    pub fn bounce(&mut self, a: Pos, b: Pos) {
        for (at, to) in [(a, b), (b, a)] {
            self.motion_mut(at).swap = Some(Swap {
                dir: [(to.x - at.x) as f32, (to.y - at.y) as f32],
                t: 0.0,
                bounce: true,
            });
        }
    }

    /// Removes the gem at `p` and blocks the cell for a moment; gems that later fall
    /// into the gap belong to cascade `chain + 1`. Returns the gem and where it was drawn.
    pub fn clear(&mut self, p: Pos, chain: u32) -> Option<(Gem, [f32; 2])> {
        let gem = self.board.get(p)?;
        let at = self.drawn_at(p);
        self.board.set(p, None);
        *self.motion_mut(p) = Motion::default();
        self.hold[idx(p)] = HOLD_SECS;
        self.hole_chain[idx(p)] = chain + 1;

        Some((gem, at))
    }

    /// Puts a new resting gem into the (cleared) cell `p`; the cell stays blocked until
    /// its hold runs out, so nothing falls through it.
    pub fn place(&mut self, p: Pos, gem: Gem) {
        self.board.set(p, Some(gem));
        *self.motion_mut(p) = Motion::default();
        self.hole_chain[idx(p)] = 0;
    }

    /// A new gem with a fresh id.
    pub fn new_gem(&mut self, color: u8, power: Power) -> Gem {
        self.board.new_gem(color, power, false)
    }

    /// Runs swaps, falls and holds for `dt` seconds. Returns how many gems landed.
    pub fn advance(&mut self, dt: f32) -> usize {
        let mut landed = 0;

        for i in 0..CELLS {
            self.hold[i] = (self.hold[i] - dt).max(0.0);
            let m = &mut self.motion[i];

            if let Some(s) = &mut m.swap {
                s.t += dt;
                if s.done() {
                    m.swap = None;
                }
            }

            if m.fall < 0.0 {
                m.vy += GRAVITY * dt;
                m.fall += m.vy * dt;

                if m.fall >= 0.0 {
                    m.fall = 0.0;
                    m.vy = 0.0;
                    landed += 1;
                }
            }
        }

        self.keep_lanes_apart();

        landed
    }

    /// A gem never overtakes the one ahead of it in its lane: it rides behind instead.
    fn keep_lanes_apart(&mut self) {
        let g = self.gravity;

        for lane in 0..SIZE {
            let mut ahead: Option<(f32, f32)> = None; // drawn depth and speed
            for d in 0..SIZE {
                let p = g.cell(lane, d);

                if self.board.get(p).is_none() {
                    continue;
                }

                let m = &mut self.motion[idx(p)];
                let drawn = d as f32 - m.fall;
                if let Some((depth, vy)) = ahead
                    && drawn < depth + 1.0
                    && m.fall < 0.0
                {
                    m.fall = d as f32 - (depth + 1.0);
                    m.vy = m.vy.min(vy);
                }

                ahead = Some((d as f32 - m.fall, m.vy));
            }
        }
    }

    /// Lets gems fall into open empty cells (blocked cells hold up what is behind them)
    /// and fills the far end of each lane with new gems coming in from beyond the edge.
    /// One in `time_odds` new gems carries a time bonus. Gems falling past a gap take on
    /// the cascade depth of the clear that made it.
    pub fn gravity_step(&mut self, rng: &mut Rng, time_odds: u32) {
        for lane in 0..SIZE {
            let (floor, farthest, chain) = self.settle_lane(lane);
            self.refill_lane(rng, time_odds, lane, floor, farthest, chain);
        }
    }

    /// Moves the gems of `lane` down to rest on each other or on a held cell. Returns the
    /// first empty depth, the drawn depth of the farthest gem or block, and the cascade
    /// chain the holes carried.
    fn settle_lane(&mut self, lane: i32) -> (i32, f32, u32) {
        let g = self.gravity;
        let mut floor = 0;
        let mut farthest = -1.0f32; // drawn depth of the farthest gem or block so far
        let mut chain = 0;

        for d in 0..SIZE {
            let p = g.cell(lane, d);

            if self.hold[idx(p)] > 0.0 {
                floor = d + 1;
                farthest = d as f32;
                chain = 0;
                continue;
            }

            let Some(gem) = self.board.get(p) else {
                chain = chain.max(self.hole_chain[idx(p)]);
                self.hole_chain[idx(p)] = 0;
                continue;
            };

            let to = g.cell(lane, floor);

            if d != floor {
                let mut m = self.motion[idx(p)];
                m.fall -= (d - floor) as f32;
                m.chain = m.chain.max(chain);
                self.board.set(to, Some(gem));
                self.board.set(p, None);
                self.motion[idx(to)] = m;
                self.motion[idx(p)] = Motion::default();
            }

            farthest = farthest.max(floor as f32 - self.motion[idx(to)].fall);
            floor += 1;
        }

        (floor, farthest, chain)
    }

    /// Fills depths `floor..` of `lane` with new gems, queued behind whatever is
    /// `farthest` out in the lane (they are empty and open past the edge).
    fn refill_lane(
        &mut self,
        rng: &mut Rng,
        time_odds: u32,
        lane: i32,
        floor: i32,
        farthest: f32,
        chain: u32,
    ) {
        let mut drawn = farthest.max(SIZE as f32 - 1.0);

        for d in floor..SIZE {
            drawn += 1.0;
            #[expect(
                clippy::cast_possible_truncation,
                reason = "below(COLORS) is less than COLORS, which is a u8"
            )]
            let color = rng.below(u32::from(COLORS)) as u8;
            let gem = self
                .board
                .new_gem(color, Power::None, rng.one_in(time_odds));
            let p = self.gravity.cell(lane, d);
            self.board.set(p, Some(gem));
            self.motion[idx(p)] = Motion {
                fall: d as f32 - drawn,
                chain,
                ..Motion::default()
            };
        }
    }

    /// Mixes the gems into a playable arrangement and drops them all in anew.
    pub fn reshuffle(&mut self, rng: &mut Rng) {
        self.board.reshuffle(rng);

        for p in Pos::all() {
            *self.motion_mut(p) = Motion {
                fall: -(SIZE as f32) - 1.0 - self.gravity.lane(p) as f32 * 0.25,
                ..Motion::default()
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Advances until nothing moves (no matching), at 120 Hz.
    fn settle(f: &mut Field, rng: &mut Rng) {
        for _ in 0..2000 {
            f.advance(1.0 / 120.0);
            f.gravity_step(rng, 0);

            if f.is_still() {
                return;
            }
        }

        panic!("never settled");
    }

    #[test]
    fn cleared_cells_refill_from_above() {
        let mut rng = Rng::new(1);
        let mut f = Field::new(Board::random(&mut rng));
        let ids: Vec<u32> = (0..SIZE)
            .map(|y| f.board().get(Pos::new(2, y)).unwrap().id)
            .collect();
        f.clear(Pos::new(2, 5), 1);
        f.clear(Pos::new(2, 6), 1);
        settle(&mut f, &mut rng);

        // The five gems above dropped two rows, two new ones sit on top.
        for (y, id) in (2..).zip(&ids[..5]) {
            assert_eq!(f.board().get(Pos::new(2, y)).unwrap().id, *id);
        }

        assert_eq!(f.board().get(Pos::new(2, 7)).unwrap().id, ids[7]);
        assert!(!ids.contains(&f.board().get(Pos::new(2, 0)).unwrap().id));
    }

    #[test]
    fn held_cells_hold_up_the_gems_above() {
        let mut rng = Rng::new(2);
        let mut f = Field::new(Board::random(&mut rng));
        f.clear(Pos::new(0, 7), 1);
        f.gravity_step(&mut rng, 0);
        assert!(f.board().get(Pos::new(0, 7)).is_none(), "still blocked");
        assert!(
            f.is_settled(Pos::new(0, 6)),
            "nothing falls through the hold"
        );
        settle(&mut f, &mut rng);
        assert!(f.board().get(Pos::new(0, 7)).is_some());
    }

    #[test]
    #[expect(
        clippy::float_cmp,
        reason = "cells are whole numbers; only pieces exactly on this lane count"
    )]
    fn falling_gems_never_overlap() {
        let mut rng = Rng::new(3);
        let mut f = Field::new(Board::random(&mut rng));

        for y in [7, 5, 3] {
            f.clear(Pos::new(4, y), 1);
        }

        for step in 0..400 {
            if step == 20 {
                f.clear(Pos::new(4, 7), 1);
            }

            f.advance(1.0 / 120.0);
            f.gravity_step(&mut rng, 0);
            let mut rows: Vec<f32> = f
                .pieces()
                .filter(|q| q.pos[0] == 4.0)
                .map(|q| q.pos[1])
                .collect();
            rows.sort_by(f32::total_cmp);

            for w in rows.windows(2) {
                assert!(w[1] - w[0] >= 0.999, "overlap at step {step}: {rows:?}");
            }
        }
    }

    #[test]
    fn every_gravity_refills_from_the_far_edge() {
        for g in [Gravity::Down, Gravity::Left, Gravity::Up, Gravity::Right] {
            let mut rng = Rng::new(5);
            let mut f = Field::new(Board::random(&mut rng));
            assert!(f.set_gravity(g));
            let lane: Vec<u32> = (0..SIZE)
                .map(|d| f.board().get(g.cell(2, d)).unwrap().id)
                .collect();
            f.clear(g.cell(2, 1), 1);
            settle(&mut f, &mut rng);
            // The gem at depth 0 stays; the ones behind the gap moved one cell closer.
            assert_eq!(f.board().get(g.cell(2, 0)).unwrap().id, lane[0], "{g:?}");

            for (d, behind) in (1..SIZE - 1).zip(&lane[2..]) {
                let id = f.board().get(g.cell(2, d)).unwrap().id;
                assert_eq!(id, *behind, "{g:?} depth {d}");
            }

            assert!(!lane.contains(&f.board().get(g.cell(2, SIZE - 1)).unwrap().id));
        }
    }

    #[test]
    fn gravity_only_turns_while_still() {
        let mut rng = Rng::new(6);
        let mut f = Field::new(Board::random(&mut rng));
        f.clear(Pos::new(1, 1), 1);
        assert!(!f.set_gravity(Gravity::Up));
        settle(&mut f, &mut rng);
        assert!(f.set_gravity(Gravity::Up));
        assert_eq!(f.gravity(), Gravity::Up);
    }
}
