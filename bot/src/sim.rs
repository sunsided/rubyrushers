//! What the bot expects a swap to do: the matches it makes, the special gems they leave,
//! the gems falling into the gaps and the cascades that follow. Gems that will fall in
//! from above are unknown, so the board fills up with [`Tile::Unknown`] and the
//! prediction only covers what can be seen now.

use crate::board::{Board, SIZE};
use crate::tile::Tile;

/// A swap of two neighbouring cells, as (x, y) pairs.
pub type Swap = ([usize; 2], [usize; 2]);

/// The predicted result of a swap.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Outcome {
    /// Gems cleared plus bonuses for special shapes, each cascade step weighted by its
    /// depth.
    pub gain: u32,
    /// The board after everything settled; unknown where new gems will be.
    pub after: Board,
}

/// Gems cleared together, with what they leave behind.
#[derive(Clone, Debug)]
struct Group {
    /// The cells.
    cells: Vec<[usize; 2]>,
    /// Points: one per gem, plus a bonus for the shape.
    value: u32,
    /// The special gem left behind, and where.
    reward: Option<(Tile, [usize; 2])>,
}

/// Predicts what swapping `a` and `b` does, or `None` if the game would not take the
/// swap (unknown tiles, nothing lines up).
pub fn play(board: &Board, (a, b): Swap) -> Option<Outcome> {
    let (ta, tb) = (at(board, a), at(board, b));
    if ta == Tile::Unknown || tb == Tile::Unknown {
        return None;
    }
    let mut now = *board;
    set(&mut now, a, tb);
    set(&mut now, b, ta);
    let mut holes = [[false; SIZE]; SIZE];
    let mut gain = match (ta, tb) {
        (Tile::Wild, Tile::Wild) => {
            holes = [[true; SIZE]; SIZE];
            40 + board
                .iter()
                .flatten()
                .filter(|t| matches!(t, Tile::Gem(_)))
                .count() as u32
        }
        (Tile::Wild, other) | (other, Tile::Wild) => {
            let mut n = 0;
            for (y, row) in now.iter().enumerate() {
                for (x, t) in row.iter().enumerate() {
                    if t.matches(other) || *t == Tile::Wild && ([x, y] == a || [x, y] == b) {
                        holes[y][x] = true;
                        n += 1;
                    }
                }
            }
            30 + n
        }
        _ => {
            let groups = groups(&now, &[a, b]);
            if groups.is_empty() {
                return None;
            }
            clear(&mut now, &mut holes, &groups)
        }
    };
    fall(&mut now, &holes);
    // Cascades among what is known.
    for depth in 2.. {
        let groups = groups(&now, &[]);
        if groups.is_empty() {
            break;
        }
        let mut holes = [[false; SIZE]; SIZE];
        gain += depth * clear(&mut now, &mut holes, &groups);
        fall(&mut now, &holes);
    }
    Some(Outcome { gain, after: now })
}

/// The tile at `p`.
fn at(board: &Board, p: [usize; 2]) -> Tile {
    board[p[1]][p[0]]
}

/// Puts `t` at `p`.
fn set(board: &mut Board, p: [usize; 2], t: Tile) {
    board[p[1]][p[0]] = t;
}

/// Marks the groups' cells as holes, puts their reward gems in, and returns their value.
fn clear(board: &mut Board, holes: &mut [[bool; SIZE]; SIZE], groups: &[Group]) -> u32 {
    for g in groups {
        for c in &g.cells {
            holes[c[1]][c[0]] = true;
        }
    }
    for g in groups {
        if let Some((tile, p)) = g.reward {
            holes[p[1]][p[0]] = false;
            set(board, p, tile);
        }
    }
    groups.iter().map(|g| g.value).sum()
}

/// Drops the gems above the holes down and fills the top with unknown gems.
fn fall(board: &mut Board, holes: &[[bool; SIZE]; SIZE]) {
    for x in 0..SIZE {
        let kept: Vec<Tile> = (0..SIZE)
            .rev()
            .filter(|&y| !holes[y][x])
            .map(|y| board[y][x])
            .collect();
        for y in (0..SIZE).rev() {
            board[y][x] = kept.get(SIZE - 1 - y).copied().unwrap_or(Tile::Unknown);
        }
    }
}

/// Every line of three or more on `board`, connected lines joined; `moved` are the cells
/// the player swapped (special gems appear there).
fn groups(board: &Board, moved: &[[usize; 2]]) -> Vec<Group> {
    // Straight runs: cells and whether horizontal.
    let mut runs: Vec<(Vec<[usize; 2]>, bool)> = Vec::new();
    for horizontal in [true, false] {
        for line in 0..SIZE {
            let cell = |i: usize| if horizontal { [i, line] } else { [line, i] };
            let mut start = 0;
            while start < SIZE {
                let t = at(board, cell(start));
                let mut end = start + 1;
                while end < SIZE && at(board, cell(end)).matches(t) {
                    end += 1;
                }
                if end - start >= 3 && matches!(t, Tile::Gem(_)) {
                    runs.push(((start..end).map(cell).collect(), horizontal));
                }
                start = end;
            }
        }
    }
    // Join runs that share a cell.
    let mut label: Vec<usize> = (0..runs.len()).collect();
    for i in 0..runs.len() {
        for j in 0..i {
            if runs[i].0.iter().any(|c| runs[j].0.contains(c)) {
                let (from, to) = (label[i], label[j]);
                label
                    .iter_mut()
                    .filter(|l| **l == from)
                    .for_each(|l| *l = to);
            }
        }
    }
    let mut roots = label.clone();
    roots.sort_unstable();
    roots.dedup();
    roots
        .into_iter()
        .map(|root| {
            let members: Vec<&(Vec<[usize; 2]>, bool)> = runs
                .iter()
                .zip(&label)
                .filter(|(_, l)| **l == root)
                .map(|(r, _)| r)
                .collect();
            let mut cells: Vec<[usize; 2]> = members.iter().flat_map(|r| r.0.clone()).collect();
            cells.sort_unstable();
            cells.dedup();
            let longest = members.iter().map(|r| r.0.len()).max().unwrap_or(0);
            let both = members.iter().any(|r| r.1) && members.iter().any(|r| !r.1);
            let color = at(board, cells[0]);
            let (bonus, reward) = if longest >= 5 {
                (10, Some(Tile::Wild))
            } else if both {
                (6, Some(color))
            } else if longest == 4 {
                (4, Some(color))
            } else {
                (0, None)
            };
            let spawn = moved
                .iter()
                .copied()
                .find(|m| cells.contains(m))
                .unwrap_or(cells[cells.len() / 2]);
            Group {
                value: cells.len() as u32 + bonus,
                reward: reward.map(|t| (t, spawn)),
                cells,
            }
        })
        .collect()
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A board from rows of digits (`W` wild); anything else is unknown.
    pub fn parse(rows: &[&str]) -> Board {
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
    fn a_match_clears_drops_and_leaves_unknowns_on_top() {
        let b = parse(&[
            "4???????", "1???????", "2???????", "3???????", "5???????", "0???????", "0???????",
            "10??????",
        ]);
        let out = play(&b, ([0, 7], [1, 7])).expect("valid");
        assert_eq!(out.gain, 3);
        // Column 0 lost (0,5), (0,6) and the swapped-in 0 at (0,7): the rest dropped three.
        let col: Vec<Tile> = (0..SIZE).map(|y| out.after[y][0]).collect();
        assert_eq!(&col[..3], &[Tile::Unknown; 3]);
        assert_eq!(col[3..], [4, 1, 2, 3, 5].map(Tile::Gem));
        assert_eq!(out.after[7][1], Tile::Gem(1));
    }

    #[test]
    fn four_leaves_a_gem_of_its_colour_where_the_player_moved() {
        let b = parse(&[
            "????????", "????????", "????????", "????????", "????????", "????????", "????????",
            "00010???",
        ]);
        let out = play(&b, ([3, 7], [4, 7])).expect("valid");
        assert_eq!(out.gain, 4 + 4);
        assert_eq!(out.after[7][3], Tile::Gem(0));
    }

    #[test]
    fn cascades_count_double() {
        // 5-5-5 in row 6 drops the two 1s of column 0 onto the 1 at the bottom.
        let b = parse(&[
            "????????", "????????", "????????", "????????", "1???????", "15??????", "545?????",
            "1???????",
        ]);
        let out = play(&b, ([1, 5], [1, 6])).expect("valid");
        assert_eq!(out.gain, 3 + 2 * 3);
        assert!((5..SIZE).all(|y| out.after[y][0] == Tile::Unknown));
    }

    #[test]
    fn unknowns_never_match_and_never_swap() {
        let b = parse(&["0?0", "?0?"]);
        assert!(play(&b, ([1, 0], [1, 1])).is_none());
    }
}
