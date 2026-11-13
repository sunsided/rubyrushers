//! One game from start to game over.

use super::board::Board;
use super::mode::Mode;
use super::moves;
use super::pos::Pos;
use super::power::Power;
use super::resolve::{Trigger, resolve};
use super::rng::Rng;
use super::step::Step;

/// A game in progress: the board, the score and the mode's clock.
#[derive(Clone, Debug)]
pub struct Session {
    /// The gems.
    board: Board,
    /// All randomness of the game.
    rng: Rng,
    /// The mode and its clock.
    mode: Mode,
    /// Points so far.
    score: u64,
    /// The deepest cascade so far.
    best_cascade: u32,
    /// Whether the game has ended.
    over: bool,
}

impl Session {
    /// A new game of `mode`, its board drawn from `seed`.
    pub fn new(mode: Mode, seed: u64) -> Self {
        let mut rng = Rng::new(seed);
        Self {
            board: Board::random(&mut rng),
            rng,
            mode,
            score: 0,
            best_cascade: 0,
            over: false,
        }
    }

    /// The gems as they are after everything resolved so far.
    pub fn board(&self) -> &Board {
        &self.board
    }

    /// The mode and its clock.
    pub fn mode(&self) -> &Mode {
        &self.mode
    }

    /// Points so far.
    pub fn score(&self) -> u64 {
        self.score
    }

    /// The deepest cascade so far.
    pub fn best_cascade(&self) -> u32 {
        self.best_cascade
    }

    /// Whether the game has ended.
    pub fn is_over(&self) -> bool {
        self.over
    }

    /// Runs the clock by `dt` seconds.
    pub fn tick(&mut self, dt: f32) {
        if !self.over && self.mode.tick(dt) {
            self.over = true;
        }
    }

    /// A move the player could make.
    pub fn hint(&self) -> Option<(Pos, Pos)> {
        moves::valid_swaps(&self.board).into_iter().next()
    }

    /// Swaps the gems at `a` and `b` and resolves everything that follows. Returns no
    /// steps when the swap is not possible at all (game over, not neighbours), a single
    /// failed [`Step::Swap`] when it lines nothing up.
    pub fn swap(&mut self, a: Pos, b: Pos) -> Vec<Step> {
        let (Some(ga), Some(gb)) = (self.board.get(a), self.board.get(b)) else {
            return Vec::new();
        };
        if self.over || !a.adjacent(b) {
            return Vec::new();
        }
        let ok = moves::is_valid(&self.board, a, b);
        let mut steps = vec![Step::Swap {
            a: (a, ga.id),
            b: (b, gb.id),
            ok,
        }];
        if !ok {
            return steps;
        }
        self.board.swap(a, b);
        // After the swap `ga` sits at `b` and `gb` at `a`.
        let trigger = match (ga.power == Power::Wild, gb.power == Power::Wild) {
            (true, true) => Trigger {
                forced: Pos::all().collect(),
                ..Trigger::default()
            },
            (true, false) => Trigger {
                forced: vec![b],
                wild_colors: vec![(b, gb.color)],
                ..Trigger::default()
            },
            (false, true) => Trigger {
                forced: vec![a],
                wild_colors: vec![(a, ga.color)],
                ..Trigger::default()
            },
            (false, false) => Trigger {
                moved: vec![a, b],
                ..Trigger::default()
            },
        };
        let resolved = resolve(
            &mut self.board,
            &mut self.rng,
            trigger,
            self.mode.time_gem_odds(),
        );
        for step in &resolved {
            if let Step::Clear(c) = step {
                self.score += c.points;
                self.best_cascade = self.best_cascade.max(c.cascade);
                self.mode.add_time(c.time_gems);
            }
        }
        steps.extend(resolved);

        if moves::valid_swaps(&self.board).is_empty() {
            match self.mode {
                Mode::Endless => self.over = true,
                Mode::Rush { .. } => steps.push(Step::Reshuffle {
                    gems: self.board.reshuffle(&mut self.rng),
                }),
            }
        }
        steps
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Plays the first hinted move `n` times.
    fn play(session: &mut Session, n: usize) -> Vec<Step> {
        let mut all = Vec::new();
        for _ in 0..n {
            let Some((a, b)) = session.hint() else { break };
            all.extend(session.swap(a, b));
        }
        all
    }

    #[test]
    fn hinted_moves_score() {
        let mut s = Session::new(Mode::Endless, 42);
        let steps = play(&mut s, 1);
        assert!(matches!(steps[0], Step::Swap { ok: true, .. }));
        assert!(s.score() > 0);
    }

    #[test]
    fn a_swap_that_lines_nothing_up_fails() {
        let mut s = Session::new(Mode::Endless, 1);
        let valid = moves::valid_swaps(s.board());
        let (a, b) = Pos::all()
            .map(|p| (p, Pos::new(p.x + 1, p.y)))
            .find(|&(a, b)| b.on_board() && !valid.contains(&(a, b)))
            .expect("some swap is invalid");
        let before = s.board().clone();
        let steps = s.swap(a, b);
        assert!(matches!(steps.as_slice(), [Step::Swap { ok: false, .. }]));
        assert!(Pos::all().all(|p| s.board().get(p) == before.get(p)));
        assert!(
            s.swap(a, Pos::new(a.x + 2, a.y)).is_empty(),
            "not neighbours"
        );
    }

    #[test]
    fn same_seed_same_game() {
        let run = || {
            let mut s = Session::new(Mode::rush(), 9);
            let steps = play(&mut s, 30);
            (steps, s.score())
        };
        assert_eq!(run(), run());
    }

    #[test]
    fn rush_ends_when_time_runs_out() {
        let mut s = Session::new(Mode::rush(), 3);
        s.tick(30.0);
        assert!(!s.is_over());
        s.tick(31.0);
        assert!(s.is_over());
        let (a, b) = moves::valid_swaps(s.board())[0];
        assert!(s.swap(a, b).is_empty(), "no moves after the end");
    }

    #[test]
    fn long_games_stay_consistent() {
        for seed in 0..5 {
            let mut s = Session::new(Mode::rush(), seed);
            play(&mut s, 200);
            assert!(Pos::all().all(|p| s.board().get(p).is_some()));
            assert!(super::super::matcher::find(s.board(), &[]).is_empty());
            assert!(
                s.hint().is_some(),
                "rush always reshuffles into a playable board"
            );
        }
    }

    #[test]
    fn endless_ends_exactly_when_stuck() {
        let mut ended = 0;
        for seed in 0..20 {
            let mut s = Session::new(Mode::Endless, seed);
            for _ in 0..2000 {
                let Some((a, b)) = s.hint() else { break };
                s.swap(a, b);
            }
            assert_eq!(s.is_over(), s.hint().is_none(), "seed {seed}");
            ended += usize::from(s.is_over());
        }
        assert!(
            ended > 0,
            "no endless game got stuck; the test proves nothing"
        );
    }
}
