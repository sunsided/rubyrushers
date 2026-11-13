//! Finding lines of three or more and what they earn.

use super::board::{Board, SIZE};
use super::pos::Pos;
use super::power::Power;

/// Gems cleared together: one line, or crossing lines of one colour.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Group {
    /// The cells, sorted.
    pub cells: Vec<Pos>,
    /// The special gem the match earns ([`Power::None`] for a plain three).
    pub reward: Power,
    /// Where the reward appears: the moved cell if it is part of the group, else the
    /// crossing of an L/T, else the middle.
    pub spawn: Pos,
}

/// A straight line of three or more matching gems.
struct Run {
    /// The cells, in order.
    cells: Vec<Pos>,
    /// Along a row (else along a column).
    horizontal: bool,
}

/// All matches on the board. `moved` are the cells the player just swapped (they get
/// the reward gems); pass none for cascades.
pub fn find(board: &Board, moved: &[Pos]) -> Vec<Group> {
    let runs = runs(board);
    // Runs that share a cell form one group (an L, T or plus).
    let mut component: Vec<usize> = (0..runs.len()).collect();
    for i in 0..runs.len() {
        for j in 0..i {
            if runs[i].cells.iter().any(|c| runs[j].cells.contains(c)) {
                let (from, to) = (component[i], component[j]);
                for k in &mut component {
                    if *k == from {
                        *k = to;
                    }
                }
            }
        }
    }
    let mut roots: Vec<usize> = component.clone();
    roots.sort_unstable();
    roots.dedup();
    roots
        .into_iter()
        .map(|root| {
            let members: Vec<&Run> = runs
                .iter()
                .zip(&component)
                .filter(|(_, c)| **c == root)
                .map(|(r, _)| r)
                .collect();
            group(&members, moved)
        })
        .collect()
}

/// Builds the group of connected runs.
fn group(members: &[&Run], moved: &[Pos]) -> Group {
    let mut cells: Vec<Pos> = members.iter().flat_map(|r| r.cells.clone()).collect();
    cells.sort_unstable();
    cells.dedup();
    let longest = members.iter().map(|r| r.cells.len()).max().unwrap_or(0);
    let both = members.iter().any(|r| r.horizontal) && members.iter().any(|r| !r.horizontal);
    let reward = if longest >= 5 {
        Power::Wild
    } else if both {
        Power::Cross
    } else if longest == 4 {
        Power::Bomb
    } else {
        Power::None
    };
    let crossing = || {
        cells.iter().copied().find(|c| {
            members.iter().any(|r| r.horizontal && r.cells.contains(c))
                && members.iter().any(|r| !r.horizontal && r.cells.contains(c))
        })
    };
    let spawn = moved
        .iter()
        .copied()
        .find(|m| cells.contains(m))
        .or_else(crossing)
        .unwrap_or(cells[cells.len() / 2]);
    Group {
        cells,
        reward,
        spawn,
    }
}

/// Every maximal straight line of three or more matching gems.
fn runs(board: &Board) -> Vec<Run> {
    let mut runs = Vec::new();
    for horizontal in [true, false] {
        for line in 0..SIZE {
            let at = |i| {
                if horizontal {
                    Pos::new(i, line)
                } else {
                    Pos::new(line, i)
                }
            };
            let mut start = 0;
            while start < SIZE {
                let mut end = start + 1;
                if let Some(first) = board.get(at(start)) {
                    while end < SIZE && board.get(at(end)).is_some_and(|g| g.matches(&first)) {
                        end += 1;
                    }
                    if end - start >= 3 && first.power != Power::Wild {
                        runs.push(Run {
                            cells: (start..end).map(at).collect(),
                            horizontal,
                        });
                    }
                }
                start = end;
            }
        }
    }
    runs
}

/// Whether a line of three or more passes through `p`.
pub fn lines_up_at(board: &Board, p: Pos) -> bool {
    let Some(gem) = board.get(p) else {
        return false;
    };
    let count = |dx: i32, dy: i32| {
        (1..)
            .take_while(|&i| {
                board
                    .get(Pos::new(p.x + dx * i, p.y + dy * i))
                    .is_some_and(|g| g.matches(&gem))
            })
            .count()
    };
    gem.power != Power::Wild && (count(-1, 0) + count(1, 0) >= 2 || count(0, -1) + count(0, 1) >= 2)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The single group on `rows`.
    fn only(rows: &[&str], moved: &[Pos]) -> Group {
        let groups = find(&Board::parse(rows), moved);
        assert_eq!(groups.len(), 1, "{groups:?}");
        groups.into_iter().next().unwrap()
    }

    #[test]
    fn three_in_a_row_earns_nothing() {
        let g = only(&["000"], &[]);
        assert_eq!(g.cells.len(), 3);
        assert_eq!(g.reward, Power::None);
    }

    #[test]
    fn four_earns_a_bomb_at_the_moved_cell() {
        let g = only(&["0000"], &[Pos::new(3, 0)]);
        assert_eq!(g.reward, Power::Bomb);
        assert_eq!(g.spawn, Pos::new(3, 0));
    }

    #[test]
    fn five_earns_a_wild() {
        let g = only(&["1", "1", "1", "1", "1"], &[]);
        assert_eq!(g.reward, Power::Wild);
        assert_eq!(g.spawn, Pos::new(0, 2));
    }

    #[test]
    fn l_and_t_earn_a_cross_at_the_corner() {
        let l = only(&["2..", "2..", "222"], &[]);
        assert_eq!(
            (l.reward, l.cells.len(), l.spawn),
            (Power::Cross, 5, Pos::new(0, 2))
        );
        let t = only(&["333", ".3.", ".3."], &[]);
        assert_eq!((t.reward, t.spawn), (Power::Cross, Pos::new(1, 0)));
    }

    #[test]
    fn separate_lines_are_separate_groups() {
        let groups = find(&Board::parse(&["000", "111"]), &[]);
        assert_eq!(groups.len(), 2);
    }

    #[test]
    fn wilds_and_gaps_break_lines() {
        assert!(find(&Board::parse(&["00W0"]), &[]).is_empty());
        assert!(find(&Board::parse(&["00.0"]), &[]).is_empty());
        assert!(find(&Board::parse(&["WWW"]), &[]).is_empty());
    }

    #[test]
    fn specials_match_by_colour() {
        let g = only(&["0A0"], &[]);
        assert_eq!(g.cells.len(), 3);
    }

    #[test]
    fn lines_up_at_agrees_with_find() {
        let board = Board::parse(&["001", "110", "000"]);
        assert!(lines_up_at(&board, Pos::new(1, 2)));
        assert!(!lines_up_at(&board, Pos::new(0, 0)));
    }
}
