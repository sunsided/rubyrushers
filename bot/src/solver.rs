//! Picking a swap: the bot's own idea of the rules, as a player knows them. Lines of
//! three or more clear; four, five and L/T shapes earn special gems; a wild clears every
//! gem of the colour it swaps with.

use crate::board::{Board, SIZE};
use crate::tile::Tile;

/// A swap of two neighbouring cells, as (x, y) pairs.
pub type Swap = ([usize; 2], [usize; 2]);

/// How much a swap is worth to the bot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Value {
    /// Gems cleared plus bonuses for special shapes.
    pub gain: u32,
    /// The lowest row involved: moves low on the board shake up more gems above them.
    pub depth: usize,
}

/// Every swap that lines something up, best first.
pub fn moves(board: &Board) -> Vec<(Swap, Value)> {
    let mut out: Vec<(Swap, Value)> = Vec::new();
    for y in 0..SIZE {
        for x in 0..SIZE {
            for (nx, ny) in [(x + 1, y), (x, y + 1)] {
                if nx < SIZE && ny < SIZE {
                    let swap = ([x, y], [nx, ny]);
                    if let Some(gain) = gain(board, swap) {
                        out.push((swap, Value { gain, depth: ny }));
                    }
                }
            }
        }
    }
    out.sort_by_key(|m| std::cmp::Reverse(m.1));
    out
}

/// The best swap, if any.
pub fn best(board: &Board) -> Option<Swap> {
    moves(board).first().map(|m| m.0)
}

/// What `swap` clears, or `None` if it lines nothing up.
fn gain(board: &Board, (a, b): Swap) -> Option<u32> {
    let (ta, tb) = (board[a[1]][a[0]], board[b[1]][b[0]]);
    if ta == Tile::Unknown || tb == Tile::Unknown {
        return None;
    }
    match (ta, tb) {
        (Tile::Wild, Tile::Wild) => return Some(64 + 40),
        (Tile::Wild, other) | (other, Tile::Wild) => {
            let same = board.iter().flatten().filter(|t| t.matches(other)).count() as u32;
            return Some(same + 30);
        }
        _ => {}
    }
    let mut after = *board;
    after[a[1]][a[0]] = tb;
    after[b[1]][b[0]] = ta;
    let total = shape(&after, a) + shape(&after, b);
    (total > 0).then_some(total)
}

/// What clears through cell `p` on `board`: the gems of its lines of three or more, with
/// bonuses for four (+4), five (+10) and crossing lines (+6).
fn shape(board: &Board, p: [usize; 2]) -> u32 {
    let t = board[p[1]][p[0]];
    let run = |dx: isize, dy: isize| -> u32 {
        let mut n = 0;
        let (mut x, mut y) = (p[0] as isize + dx, p[1] as isize + dy);
        while (0..SIZE as isize).contains(&x)
            && (0..SIZE as isize).contains(&y)
            && board[y as usize][x as usize].matches(t)
        {
            n += 1;
            x += dx;
            y += dy;
        }
        n
    };
    let h = 1 + run(-1, 0) + run(1, 0);
    let v = 1 + run(0, -1) + run(0, 1);
    let line = |len: u32| match len {
        0..=2 => 0,
        3 => 3,
        4 => 4 + 4,
        _ => len + 10,
    };
    let (lh, lv) = (line(h), line(v));
    // The crossing gem counts in both lines once too often; the bonus makes up for it.
    let cross = if lh > 0 && lv > 0 { 5 } else { 0 };
    lh + lv + cross
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A board from rows of digits (`W` wild, `?` unknown); missing cells are unknown.
    fn parse(rows: &[&str]) -> Board {
        let mut b = [[Tile::Unknown; SIZE]; SIZE];
        for (y, row) in rows.iter().enumerate() {
            for (x, ch) in row.chars().enumerate() {
                b[y][x] = match ch {
                    '0'..='6' => Tile::Gem(ch as u8 - b'0'),
                    'W' => Tile::Wild,
                    _ => Tile::Unknown,
                };
            }
        }
        b
    }

    #[test]
    fn finds_the_only_move() {
        let b = parse(&["0010", "2345", "4523"]);
        assert_eq!(best(&b), Some(([2, 0], [3, 0])));
    }

    #[test]
    fn prefers_bigger_matches() {
        // Swapping (2, 1) up makes five in row 0.
        let b = parse(&["001003", "220445", "131313"]);
        let (swap, value) = moves(&b)[0];
        assert_eq!(swap, ([2, 0], [2, 1]));
        assert_eq!(value.gain, 15);
    }

    #[test]
    fn wilds_always_move_and_unknowns_never() {
        let b = parse(&["W1", "23"]);
        assert_eq!(moves(&b).len(), 2);
        let b = parse(&["??", "??"]);
        assert!(moves(&b).is_empty());
    }

    #[test]
    fn prefers_lower_moves_on_a_tie() {
        let b = parse(&["0010", "2345", "4523", "5545"]);
        assert_eq!(best(&b), Some(([2, 3], [3, 3])));
    }
}
