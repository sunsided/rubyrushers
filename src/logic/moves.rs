//! Which swaps the player can make.

use super::board::{Board, SIZE};
use super::matcher;
use super::pos::Pos;
use super::power::Power;

/// Whether swapping the gems at adjacent cells `a` and `b` is a move: it lines up three,
/// or one of them is wild.
pub fn is_valid(board: &Board, a: Pos, b: Pos) -> bool {
    let (Some(ga), Some(gb)) = (board.get(a), board.get(b)) else {
        return false;
    };
    if !a.adjacent(b) {
        return false;
    }
    if ga.power == Power::Wild || gb.power == Power::Wild {
        return true;
    }
    let mut after = board.clone();
    after.swap(a, b);
    matcher::lines_up_at(&after, a) || matcher::lines_up_at(&after, b)
}

/// Every valid swap, each pair once (left before right, top before bottom).
pub fn valid_swaps(board: &Board) -> Vec<(Pos, Pos)> {
    Pos::all()
        .flat_map(|p| [(p, Pos::new(p.x + 1, p.y)), (p, Pos::new(p.x, p.y + 1))])
        .filter(|&(a, b)| b.x < SIZE && b.y < SIZE && is_valid(board, a, b))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::rng::Rng;

    #[test]
    fn valid_swaps_match_brute_force() {
        for seed in 0..20 {
            let board = Board::random(&mut Rng::new(seed));
            for a in Pos::all() {
                for b in [Pos::new(a.x + 1, a.y), Pos::new(a.x, a.y + 1)] {
                    if !b.on_board() {
                        continue;
                    }
                    let mut after = board.clone();
                    after.swap(a, b);
                    let brute = !matcher::find(&after, &[]).is_empty();
                    assert_eq!(is_valid(&board, a, b), brute, "seed {seed} {a:?}-{b:?}");
                }
            }
        }
    }

    #[test]
    fn wild_swaps_are_always_valid() {
        let board = Board::parse(&["W1", "23"]);
        assert!(is_valid(&board, Pos::new(0, 0), Pos::new(1, 0)));
        assert!(is_valid(&board, Pos::new(0, 0), Pos::new(0, 1)));
        assert!(!is_valid(&board, Pos::new(1, 0), Pos::new(1, 1)));
    }

    #[test]
    fn a_dead_board_has_no_moves() {
        let board = Board::parse(&[
            "01234560", "12345601", "23456012", "34560123", "45601234", "56012345", "60123456",
            "01234560",
        ]);
        assert!(valid_swaps(&board).is_empty());
    }
}
