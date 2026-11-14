//! The grid of gems.

use super::gem::{COLORS, Gem};
use super::pos::Pos;
use super::power::Power;
use super::rng::Rng;
use super::{matcher, moves};

/// Cells per side.
pub const SIZE: i32 = 8;

/// An 8x8 grid of gems; cells are empty only while a move resolves.
#[derive(Clone, Debug)]
pub struct Board {
    /// Row-major cells.
    cells: [Option<Gem>; (SIZE * SIZE) as usize],
    /// The id the next new gem gets.
    next_id: u32,
}

impl Board {
    /// A board without gems.
    pub fn empty() -> Self {
        Self {
            cells: [None; (SIZE * SIZE) as usize],
            next_id: 0,
        }
    }

    /// A full board with no match on it and at least one move.
    pub fn random(rng: &mut Rng) -> Self {
        let mut board = Self::empty();
        loop {
            board.fill_without_matches(rng);
            if !moves::valid_swaps(&board).is_empty() {
                return board;
            }
        }
    }

    /// The row-major index of an on-board cell.
    fn index(p: Pos) -> usize {
        debug_assert!(p.on_board(), "{p:?} is off the board");
        (p.y * SIZE + p.x) as usize
    }

    /// The gem at `p`; `None` for an empty cell or one off the board.
    pub fn get(&self, p: Pos) -> Option<Gem> {
        if p.on_board() {
            self.cells[Self::index(p)]
        } else {
            None
        }
    }

    /// Puts `gem` (or nothing) at `p`.
    pub fn set(&mut self, p: Pos, gem: Option<Gem>) {
        self.cells[Self::index(p)] = gem;
    }

    /// Exchanges the contents of two cells.
    pub fn swap(&mut self, a: Pos, b: Pos) {
        self.cells.swap(Self::index(a), Self::index(b));
    }

    /// A new gem with a fresh id (not yet placed).
    pub fn new_gem(&mut self, color: u8, power: Power, time_bonus: bool) -> Gem {
        self.next_id += 1;
        Gem {
            id: self.next_id,
            color,
            power,
            time_bonus,
        }
    }

    /// Replaces every cell with a random plain gem so that no three line up.
    fn fill_without_matches(&mut self, rng: &mut Rng) {
        for p in Pos::all() {
            self.set(p, None);
        }
        for p in Pos::all() {
            let mut colors: Vec<u8> = (0..COLORS).collect();
            rng.shuffle(&mut colors);
            let color = colors
                .into_iter()
                .find(|&c| !self.would_line_up(p, c))
                .expect("seven colours always leave one that does not line up");
            let gem = self.new_gem(color, Power::None, false);
            self.set(p, Some(gem));
        }
    }

    /// Whether colour `c` at `p` would complete three with the two cells to its left or
    /// the two above (the cells filled before it).
    fn would_line_up(&self, p: Pos, c: u8) -> bool {
        let same = |dx, dy| {
            self.get(Pos::new(p.x + dx, p.y + dy))
                .is_some_and(|g| g.power != Power::Wild && g.color == c)
        };
        (same(-1, 0) && same(-2, 0)) || (same(0, -1) && same(0, -2))
    }

    /// Mixes the gems up until no three line up and a move exists; falls back to a new
    /// random board. Returns every gem with its new cell.
    pub fn reshuffle(&mut self, rng: &mut Rng) -> Vec<(Gem, Pos)> {
        let mut gems: Vec<Gem> = Pos::all().filter_map(|p| self.get(p)).collect();
        let shuffled = (0..100).any(|_| {
            rng.shuffle(&mut gems);
            for (p, g) in Pos::all().zip(&gems) {
                self.set(p, Some(*g));
            }
            matcher::find(self, &[]).is_empty() && !moves::valid_swaps(self).is_empty()
        });
        if !shuffled {
            let next_id = self.next_id;
            *self = Self::random(rng);
            // Keep ids unique across the replacement.
            for p in Pos::all() {
                let mut g = self.get(p).expect("random boards are full");
                g.id += next_id;
                self.set(p, Some(g));
            }
            self.next_id += next_id;
        }
        Pos::all()
            .map(|p| (self.get(p).expect("full after reshuffle"), p))
            .collect()
    }

    /// A board from rows of text, for tests. Each cell is one character: `0`-`6` a plain gem, `A`-`G` a bomb of colour 0-6,
    /// `a`-`g` a cross of colour 0-6, `W` a wild, `.` empty. Missing rows are empty.
    #[cfg(test)]
    pub fn parse(rows: &[&str]) -> Self {
        let mut board = Self::empty();
        for (y, row) in rows.iter().enumerate() {
            for (x, ch) in row.chars().enumerate() {
                let (color, power) = match ch {
                    '0'..='6' => (ch as u8 - b'0', Power::None),
                    'A'..='G' => (ch as u8 - b'A', Power::Bomb),
                    'a'..='g' => (ch as u8 - b'a', Power::Cross),
                    'W' => (0, Power::Wild),
                    _ => continue,
                };
                let gem = board.new_gem(color, power, false);
                board.set(Pos::new(x as i32, y as i32), Some(gem));
            }
        }
        board
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_board_is_full_without_matches_and_has_a_move() {
        for seed in 0..50 {
            let board = Board::random(&mut Rng::new(seed));
            assert!(Pos::all().all(|p| board.get(p).is_some()));
            assert!(matcher::find(&board, &[]).is_empty(), "seed {seed}");
            assert!(!moves::valid_swaps(&board).is_empty(), "seed {seed}");
        }
    }

    #[test]
    fn reshuffle_keeps_the_gems() {
        let mut board = Board::random(&mut Rng::new(3));
        let mut before: Vec<u32> = Pos::all().map(|p| board.get(p).unwrap().id).collect();
        let mut after: Vec<u32> = board
            .reshuffle(&mut Rng::new(4))
            .iter()
            .map(|(g, _)| g.id)
            .collect();
        before.sort_unstable();
        after.sort_unstable();
        assert_eq!(before, after);
        assert!(matcher::find(&board, &[]).is_empty());
    }
}
