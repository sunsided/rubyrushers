//! One game from start to game over, in real time.

use std::collections::BTreeSet;

use super::blast;
use super::board::Board;
use super::event::{Clear, Event};
use super::field::{Field, Piece};
use super::gravity::Gravity;
use super::matcher::{self, Group};
use super::mode::Mode;
use super::moves;
use super::pos::Pos;
use super::power::Power;
use super::rng::Rng;
use super::score;

/// A game in progress: the moving board, the score and the mode's clock.
#[derive(Clone, Debug)]
pub struct Session {
    /// The gems and their motion.
    field: Field,
    /// All randomness of the game.
    rng: Rng,
    /// The mode and its clock.
    mode: Mode,
    /// Points so far.
    score: u64,
    /// The deepest cascade so far.
    best_cascade: u32,
    /// The speed streak: matches of the player's in quick succession.
    speed: u32,
    /// Seconds since the player's last match.
    since_match: f32,
    /// Swapped wild gems waiting to land: their id and the colour they take out (`None`:
    /// every gem).
    armed: Vec<(u32, Option<u8>)>,
    /// The clock ran out; the board finishes moving, then the game ends.
    time_up: bool,
    /// Whether the game has ended.
    over: bool,
    /// Events since the last [`Session::drain_events`].
    events: Vec<Event>,
}

impl Session {
    /// A new game of `mode`, its board drawn from `seed`.
    pub fn new(mode: Mode, seed: u64) -> Self {
        let mut rng = Rng::new(seed);
        Self {
            field: Field::new(Board::random(&mut rng)),
            rng,
            mode,
            score: 0,
            best_cascade: 0,
            speed: 0,
            since_match: f32::INFINITY,
            armed: Vec::new(),
            time_up: false,
            over: false,
            events: Vec::new(),
        }
    }

    /// The gems by cell, moving ones included.
    pub fn board(&self) -> &Board {
        self.field.board()
    }

    /// Every gem as drawn.
    pub fn pieces(&self) -> impl Iterator<Item = Piece> + '_ {
        self.field.pieces()
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

    /// The speed streak (0: none).
    pub fn speed(&self) -> u32 {
        self.speed
    }

    /// How much of the speed streak's window is left, `1.0..=0.0`.
    pub fn speed_left(&self) -> f32 {
        if self.speed == 0 {
            0.0
        } else {
            (1.0 - self.since_match / score::SPEED_WINDOW).clamp(0.0, 1.0)
        }
    }

    /// Whether the game has ended.
    pub fn is_over(&self) -> bool {
        self.over
    }

    /// Whether nothing on the board moves.
    pub fn is_still(&self) -> bool {
        self.field.is_still() && self.armed.is_empty()
    }

    /// The events since the last call.
    pub fn drain_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }

    /// A move the player could make, while the board is still.
    pub fn hint(&self) -> Option<(Pos, Pos)> {
        if !self.is_still() {
            return None;
        }
        moves::valid_swaps(self.field.board()).into_iter().next()
    }

    /// Which way gems fall.
    pub fn gravity(&self) -> Gravity {
        self.field.gravity()
    }

    /// Whether the player may pick up the gem at `p` now (not while a due gravity turn
    /// waits for the board to rest).
    pub fn can_move(&self, p: Pos) -> bool {
        !self.over && !self.time_up && !self.mode.turn_due() && self.field.is_settled(p)
    }

    /// The player swaps the gems at `a` and `b`. Returns false when that is not possible
    /// now (not neighbours, a gem still moving, the game over). A swap that lines nothing
    /// up bounces back and still returns true.
    pub fn swap(&mut self, a: Pos, b: Pos) -> bool {
        if !a.adjacent(b) || !self.can_move(a) || !self.can_move(b) {
            return false;
        }
        let board = self.field.board();
        let (Some(ga), Some(gb)) = (board.get(a), board.get(b)) else {
            return false;
        };
        if !moves::is_valid(board, a, b) {
            self.field.bounce(a, b);
            self.events.push(Event::Rejected);
            return true;
        }
        self.field.start_swap(a, b);
        self.events.push(Event::Swapped);
        match (ga.power == Power::Wild, gb.power == Power::Wild) {
            (true, true) => self.armed.push((ga.id, None)),
            (true, false) => self.armed.push((ga.id, Some(gb.color))),
            (false, true) => self.armed.push((gb.id, Some(ga.color))),
            (false, false) => {}
        }
        true
    }

    /// Advances the game by `dt` seconds: the clock, the motion, and every match among
    /// the gems at rest.
    pub fn update(&mut self, dt: f32) {
        if self.over {
            return;
        }
        if self.mode.tick(dt) {
            self.time_up = true;
        }
        self.since_match += dt;
        if self.since_match > score::SPEED_WINDOW {
            self.speed = 0;
        }
        if self.field.advance(dt) > 0 {
            self.events.push(Event::Landed);
        }
        self.field
            .gravity_step(&mut self.rng, self.mode.time_gem_odds());
        self.fire_wilds();
        self.match_settled();
        self.turn();
        self.when_still();
    }

    /// Turns gravity once it is due and the board rests; the gems then fall the new way.
    fn turn(&mut self) {
        if self.mode.turn_due() && self.armed.is_empty() {
            let g = self.field.gravity().turned();
            if self.field.set_gravity(g) {
                self.mode.turned();
                self.events.push(Event::Tilted(g));
            }
        }
    }

    /// Sets off swapped wild gems that have landed.
    fn fire_wilds(&mut self) {
        let armed = std::mem::take(&mut self.armed);
        for (id, color) in armed {
            let found = Pos::all().find(|p| self.board().get(*p).is_some_and(|g| g.id == id));
            let Some(at) = found else {
                continue; // Taken out by something else meanwhile.
            };
            if !self.field.is_settled(at) {
                self.armed.push((id, color));
                continue;
            }
            let (doomed, colors) = match color {
                Some(c) => ([at].into(), vec![(at, c)]),
                None => (
                    Pos::all()
                        .filter(|p| self.board().get(*p).is_some())
                        .collect(),
                    Vec::new(),
                ),
            };
            self.clear(doomed, &[], &colors, 1);
        }
    }

    /// Clears every match among the gems at rest; resting gems that did not match forget
    /// how they got there.
    fn match_settled(&mut self) {
        let settled = self.field.settled_board();
        let moved: Vec<Pos> = Pos::all()
            .filter(|p| settled.get(*p).is_some() && self.field.motion(*p).moved)
            .collect();
        let groups = matcher::find(&settled, &moved);
        if !groups.is_empty() {
            let doomed: BTreeSet<Pos> = groups.iter().flat_map(|g| g.cells.clone()).collect();
            let cascade = doomed
                .iter()
                .map(|p| self.field.motion(*p).chain)
                .max()
                .unwrap_or(0)
                .max(1);
            self.clear(doomed, &groups, &[], cascade);
        }
        for p in Pos::all() {
            if self.field.is_settled(p) {
                let m = self.field.motion_mut(p);
                m.chain = 0;
                m.moved = false;
            }
        }
    }

    /// Clears `doomed` and whatever the special gems among them take out, places the
    /// matches' reward gems and scores it all as cascade depth `cascade`.
    fn clear(
        &mut self,
        mut doomed: BTreeSet<Pos>,
        groups: &[Group],
        wild_colors: &[(Pos, u8)],
        cascade: u32,
    ) {
        let blasts = blast::detonate(self.field.board(), &mut self.rng, &mut doomed, wild_colors);
        let reward_colors: Vec<u8> = groups
            .iter()
            .map(|g| self.board().get(g.spawn).map_or(0, |gem| gem.color))
            .collect();
        let cleared: Vec<_> = doomed
            .iter()
            .filter_map(|&p| self.field.clear(p, cascade))
            .collect();
        let gone: Vec<u32> = cleared.iter().map(|(g, _)| g.id).collect();
        self.armed.retain(|(id, _)| !gone.contains(id));

        let mut created = Vec::new();
        for (g, color) in groups.iter().zip(reward_colors) {
            if g.reward != Power::None {
                let gem = self.field.new_gem(color, g.reward);
                self.field.place(g.spawn, gem);
                created.push((gem, g.spawn));
            }
        }
        let rewards: Vec<Power> = groups.iter().map(|g| g.reward).collect();
        if cascade == 1 {
            if self.since_match <= score::SPEED_WINDOW {
                self.speed = (self.speed + 1).min(score::MAX_SPEED);
            }
            self.since_match = 0.0;
        }
        let base = score::points(cleared.len(), &rewards, blasts.len(), cascade);
        let points = score::with_speed(base, self.speed);
        let time_gems = cleared.iter().filter(|(g, _)| g.time_bonus).count() as u32;
        self.score += points;
        self.best_cascade = self.best_cascade.max(cascade);
        self.mode.add_time(time_gems);
        self.events.push(Event::Cleared(Clear {
            cascade,
            cleared,
            blasts,
            created,
            points,
            speed: self.speed,
            time_gems,
        }));
    }

    /// Once the board rests: ends the game when time is up or (endless) no move is left,
    /// or reshuffles a stuck board.
    fn when_still(&mut self) {
        if !self.is_still() {
            return;
        }
        let stuck = moves::valid_swaps(self.field.board()).is_empty();
        if self.time_up || (stuck && matches!(self.mode, Mode::Endless)) {
            self.over = true;
            self.events.push(Event::Over);
        } else if stuck {
            self.field.reshuffle(&mut self.rng);
            self.events.push(Event::Reshuffled);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::SIZE;

    /// Seconds per test tick.
    const DT: f32 = 1.0 / 120.0;

    /// Updates until the board rests or the game ends; returns the events.
    fn settle(s: &mut Session) -> Vec<Event> {
        let mut events = Vec::new();
        for _ in 0..120 * 60 {
            s.update(DT);
            events.extend(s.drain_events());
            if s.is_still() || s.is_over() {
                return events;
            }
        }
        panic!("the board never came to rest");
    }

    /// The board rests, is full and has no match on it.
    fn assert_sound(s: &Session) {
        assert!(s.is_still());
        assert!(Pos::all().all(|p| s.board().get(p).is_some()));
        assert!(matcher::find(s.board(), &[]).is_empty());
    }

    #[test]
    fn a_hinted_move_scores() {
        let mut s = Session::new(Mode::Endless, 42);
        let (a, b) = s.hint().unwrap();
        assert!(s.swap(a, b));
        let events = settle(&mut s);
        assert_eq!(events[0], Event::Swapped);
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::Cleared(c) if c.cascade == 1))
        );
        assert!(s.score() > 0);
        assert_sound(&s);
    }

    #[test]
    fn a_swap_that_lines_nothing_up_bounces_back() {
        let mut s = Session::new(Mode::Endless, 1);
        let valid = moves::valid_swaps(s.board());
        let (a, b) = Pos::all()
            .map(|p| (p, Pos::new(p.x + 1, p.y)))
            .find(|&(a, b)| b.on_board() && !valid.contains(&(a, b)))
            .expect("some swap is invalid");
        let before = s.board().clone();
        assert!(s.swap(a, b));
        assert!(!s.swap(a, b), "the gems are busy bouncing");
        let events = settle(&mut s);
        assert_eq!(events, vec![Event::Rejected]);
        assert!(Pos::all().all(|p| s.board().get(p) == before.get(p)));
        assert!(!s.swap(a, Pos::new(a.x + 2, a.y)), "not neighbours");
    }

    #[test]
    fn moving_gems_cannot_be_swapped() {
        let mut s = Session::new(Mode::Endless, 4);
        let (a, b) = s.hint().unwrap();
        s.swap(a, b);
        s.update(DT);
        assert!(!s.can_move(a) && !s.can_move(b));
    }

    /// Plays like an impatient human: every few ticks tries a random swap that lines
    /// something up, mostly while the board is still moving.
    fn frantic(s: &mut Session, ticks: usize, seed: u64) -> usize {
        let mut rng = Rng::new(seed);
        let mut busy_swaps = 0;
        for t in 0..ticks {
            if t % 3 == 0 {
                let a = Pos::new(rng.below(SIZE as u32) as i32, rng.below(SIZE as u32) as i32);
                let b = if rng.one_in(2) {
                    Pos::new(a.x + 1, a.y)
                } else {
                    Pos::new(a.x, a.y + 1)
                };
                let moving = !s.is_still();
                if b.on_board() && moves::is_valid(s.board(), a, b) && s.swap(a, b) && moving {
                    busy_swaps += 1;
                }
            }
            s.update(DT);
            s.drain_events();
            assert_apart(s);
        }
        busy_swaps
    }

    /// No two gems that are not swapping overlap in a column.
    fn assert_apart(s: &Session) {
        for x in 0..SIZE {
            let mut rows: Vec<f32> = s
                .pieces()
                .filter(|q| !q.swapping && q.pos[0] == x as f32)
                .map(|q| q.pos[1])
                .collect();
            rows.sort_by(f32::total_cmp);
            for w in rows.windows(2) {
                assert!(w[1] - w[0] >= 0.999, "column {x} overlaps: {rows:?}");
            }
        }
    }

    #[test]
    fn swapping_during_cascades_never_jams_the_board() {
        let mut busy = 0;
        for seed in 0..12 {
            let mut s = Session::new(Mode::Endless, seed);
            busy += frantic(&mut s, 120 * 30, seed + 100);
            if !s.is_over() {
                settle(&mut s);
            }
            assert_sound(&s);
            for q in s.pieces() {
                assert_eq!(
                    q.pos,
                    q.pos.map(f32::round),
                    "seed {seed}: gem between cells"
                );
            }
        }
        assert!(busy > 50, "only {busy} swaps landed while the board moved");
    }

    #[test]
    fn cascades_happen_and_count_up() {
        let best = (0..10)
            .map(|seed| {
                let mut s = Session::new(Mode::rush(), seed);
                for _ in 0..40 {
                    let Some((a, b)) = s.hint() else { break };
                    s.swap(a, b);
                    settle(&mut s);
                }
                s.best_cascade()
            })
            .max()
            .unwrap();
        assert!(best >= 2, "no cascade in 400 moves");
    }

    #[test]
    fn same_seed_same_game() {
        let run = || {
            let mut s = Session::new(Mode::rush(), 9);
            frantic(&mut s, 120 * 10, 5);
            (s.score(), s.board().get(Pos::new(3, 3)))
        };
        assert_eq!(run(), run());
    }

    #[test]
    fn rush_ends_once_time_is_up_and_the_board_rests() {
        let mut s = Session::new(Mode::rush(), 3);
        for _ in 0..(120 * 59) {
            s.update(DT);
        }
        assert!(!s.is_over());
        let (a, b) = s.hint().unwrap();
        s.swap(a, b);
        for _ in 0..(120 * 2) {
            s.update(DT);
        }
        assert!(
            !s.can_move(Pos::new(0, 0)),
            "no moves after the clock ran out"
        );
        settle(&mut s);
        assert!(s.is_over());
    }

    #[test]
    fn a_wild_swap_clears_the_partners_colour() {
        let mut s = Session::new(Mode::Endless, 6);
        let wild_at = Pos::new(3, 3);
        let partner = Pos::new(4, 3);
        let mut board = s.board().clone();
        let mut wild = board.get(wild_at).unwrap();
        wild.power = Power::Wild;
        board.set(wild_at, Some(wild));
        s.field = Field::new(board);
        let color = s.board().get(partner).unwrap().color;
        let count = Pos::all()
            .filter(|p| {
                s.board()
                    .get(*p)
                    .is_some_and(|g| g.color == color && g.power == Power::None)
            })
            .count();
        assert!(s.swap(wild_at, partner));
        let events = settle(&mut s);
        let first = events.iter().find_map(|e| match e {
            Event::Cleared(c) => Some(c),
            _ => None,
        });
        assert_eq!(first.expect("nothing cleared").cleared.len(), count + 1);
        assert_sound(&s);
    }

    #[test]
    fn endless_ends_exactly_when_stuck() {
        let mut ended = 0;
        for seed in 0..20 {
            let mut s = Session::new(Mode::Endless, seed);
            for _ in 0..400 {
                let Some((a, b)) = s.hint() else { break };
                s.swap(a, b);
                settle(&mut s);
            }
            let stuck = moves::valid_swaps(s.board()).is_empty();
            assert_eq!(s.is_over(), stuck, "seed {seed}");
            ended += usize::from(s.is_over());
        }
        assert!(
            ended > 0,
            "no endless game got stuck; the test proves nothing"
        );
    }

    #[test]
    fn rush_reshuffles_a_stuck_board() {
        for seed in 0..5 {
            // A clock that outlasts the test.
            let mut s = Session::new(Mode::Rush { remaining: 1e9 }, seed);
            for _ in 0..200 {
                let Some((a, b)) = s.hint() else { break };
                s.swap(a, b);
                settle(&mut s);
            }
            assert!(!s.is_over());
            assert!(s.hint().is_some());
        }
    }

    #[test]
    fn tilt_turns_gravity_and_keeps_the_board_sound() {
        let mut s = Session::new(Mode::tilt(), 8);
        let mut turns = Vec::new();
        let mut rng = Rng::new(3);
        for t in 0..(120 * 50) {
            if t % 4 == 0 {
                let a = Pos::new(rng.below(SIZE as u32) as i32, rng.below(SIZE as u32) as i32);
                let b = Pos::new(a.x + 1, a.y);
                if b.on_board() && moves::is_valid(s.board(), a, b) {
                    s.swap(a, b);
                }
            }
            s.update(DT);
            for e in s.drain_events() {
                if let Event::Tilted(g) = e {
                    turns.push(g);
                }
            }
            assert_apart_along(&s);
        }
        assert_eq!(turns, vec![Gravity::Left, Gravity::Up, Gravity::Right]);
        settle(&mut s);
        if !s.is_over() {
            assert_sound(&s);
        }
    }

    /// No two gems that are not swapping overlap in a lane of the current gravity.
    fn assert_apart_along(s: &Session) {
        let horizontal = matches!(s.gravity(), Gravity::Left | Gravity::Right);
        let (along, across) = if horizontal { (0, 1) } else { (1, 0) };
        for lane in 0..SIZE {
            let mut at: Vec<f32> = s
                .pieces()
                .filter(|q| !q.swapping && q.pos[across] == lane as f32)
                .map(|q| q.pos[along])
                .collect();
            at.sort_by(f32::total_cmp);
            for w in at.windows(2) {
                assert!(w[1] - w[0] >= 0.999, "lane {lane} overlaps: {at:?}");
            }
        }
    }

    #[test]
    fn quick_moves_build_speed_and_pausing_loses_it() {
        let mut s = Session::new(Mode::Endless, 11);
        for _ in 0..3 {
            let (a, b) = s.hint().unwrap();
            s.swap(a, b);
            settle(&mut s);
        }
        assert!(s.speed() >= 1, "three quick moves, speed {}", s.speed());
        for _ in 0..(120 * 3) {
            s.update(DT);
        }
        assert_eq!(s.speed(), 0);
        assert_eq!(s.speed_left(), 0.0);
    }
}
