//! Picking a swap: every move the game would take, valued by what it clears now
//! (cascades included, see [`sim`]) plus, discounted, the best move it sets up on the
//! predicted board. A smaller match now can win when it lines up a big one next.
//!
//! Where running out of moves ends the game, a swap also earns a bonus for the moves it
//! leaves open: `mobility * ln(1 + moves left)`. The steps shrink as moves get plenty,
//! so the bonus matters most when few are left, yet it steers well before that.

use crate::board::{Board, SIZE};
use crate::sim;
pub use crate::sim::Swap;

/// How far ahead the bot looks: its own move plus this many more.
pub const LOOKAHEAD: u32 = 1;
/// What a follow-up move is worth compared with one now (it may never happen).
const DISCOUNT: f32 = 0.7;
/// The mobility weight for games that end when no move is left (Endless): going from no
/// move left to one is worth about ten points, a match of five; from ten to eleven, about
/// one. Tried on four seeds: with 15 every game outlasted three minutes, with 6 or none
/// most ended within one.
pub const MOBILITY: f32 = 15.0;

/// How much a swap is worth to the bot.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Value {
    /// Gems cleared plus bonuses for special shapes, cascades included.
    pub gain: u32,
    /// `gain` plus the discounted value of the best follow-up on the predicted board, plus
    /// the mobility bonus.
    pub total: f32,
    /// Moves the bot can see on the predicted board (when looking ahead at all).
    pub left: Option<usize>,
    /// The lowest row involved: moves low on the board shake up more gems above them.
    pub depth: usize,
}

/// Every swap the game would take, with its value looking `ahead` moves further and
/// weighing the moves it leaves open by `mobility`; best first.
pub fn moves(board: &Board, ahead: u32, mobility: f32) -> Vec<(Swap, Value)> {
    let mut out: Vec<(Swap, Value)> = swaps()
        .filter_map(|swap| {
            let o = sim::play(board, swap)?;
            Some((swap, value(swap, &o, ahead, mobility)))
        })
        .collect();

    out.sort_by(|a, b| {
        b.1.total
            .total_cmp(&a.1.total)
            .then(b.1.depth.cmp(&a.1.depth))
    });

    out
}

/// What `swap` with the predicted `outcome` is worth, looking `ahead` moves further and
/// weighing the moves it leaves open by `mobility`.
fn value(swap: Swap, outcome: &sim::Outcome, ahead: u32, mobility: f32) -> Value {
    let (next, left) = if ahead > 0 {
        let follow = moves(&outcome.after, ahead - 1, mobility);
        (
            follow.first().map_or(0.0, |m| m.1.total),
            Some(follow.len()),
        )
    } else {
        (0.0, None)
    };

    let open = left.map_or(0.0, |n| mobility * (1.0 + n as f32).ln());

    Value {
        gain: outcome.gain,
        total: outcome.gain as f32 + DISCOUNT * next + open,
        left,
        depth: swap.1[1],
    }
}

/// The best swap, looking [`LOOKAHEAD`] moves ahead, without minding mobility.
pub fn best(board: &Board) -> Option<Swap> {
    moves(board, LOOKAHEAD, 0.0).first().map(|m| m.0)
}

/// Every pair of neighbouring cells, each once.
fn swaps() -> impl Iterator<Item = Swap> {
    (0..SIZE).flat_map(|y| {
        (0..SIZE).flat_map(move |x| {
            [(x + 1, y), (x, y + 1)]
                .into_iter()
                .filter(|&(nx, ny)| nx < SIZE && ny < SIZE)
                .map(move |(nx, ny)| ([x, y], [nx, ny]))
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::tests::parse;
    use crate::tile::Tile;

    #[test]
    fn finds_the_only_move() {
        let b = parse(&["0010", "2345", "4523"]);
        assert_eq!(best(&b), Some(([2, 0], [3, 0])));
    }

    #[test]
    fn prefers_bigger_matches() {
        // Swapping (2, 1) up makes five in row 0.
        let b = parse(&["001003", "220445", "131313"]);

        let (swap, value) = moves(&b, 0, 0.0)[0];
        assert_eq!(swap, ([2, 0], [2, 1]));
        assert_eq!(value.gain, 15);
    }

    #[test]
    fn wilds_always_move_and_unknowns_never() {
        assert_eq!(moves(&parse(&["W1", "23"]), 0, 0.0).len(), 2);
        assert!(moves(&parse(&["??", "??"]), 0, 0.0).is_empty());
    }

    #[test]
    fn prefers_lower_moves_on_a_tie() {
        let b = parse(&["0010", "2345", "4523", "5545"]);
        assert_eq!(moves(&b, 0, 0.0)[0].0, ([2, 3], [3, 3]));

        // Looking ahead, the top move wins: it leaves the bottom one standing, while the
        // bottom one drops the top row out of reach.
        assert_eq!(best(&b), Some(([2, 0], [3, 0])));
    }

    #[test]
    fn looks_ahead_past_the_greedy_move() {
        // Greedy: the 4-4-4 at the bottom right (lowest). Better: the 5-5-5 in column 2
        // drops the 0 from (2, 3) onto (2, 6), and swapping it down then makes 0-0-0-0-0
        // in the bottom row.
        let b = parse(&[
            "????????", "????????", "????????", "??0?????", "??5?????", "??5?????", "??45?4??",
            "00100344",
        ]);

        let greedy = moves(&b, 0, 0.0)[0].0;
        assert_eq!(greedy, ([5, 6], [5, 7]));

        assert_eq!(best(&b), Some(([2, 6], [3, 6])));
        assert!(b.iter().flatten().any(|t| *t == Tile::Unknown));
    }

    /// A busy board, from rows of digits.
    fn busy_board() -> Board {
        parse(&[
            "01234501", "23450123", "45012345", "01234501", "10325410", "32104532", "54321054",
            "00110022",
        ])
    }

    #[test]
    fn counts_the_moves_left_on_the_predicted_board() {
        let b = busy_board();
        for (swap, value) in moves(&b, 1, 0.0) {
            let after = sim::play(&b, swap).expect("listed moves are valid").after;
            assert_eq!(value.left, Some(moves(&after, 0, 0.0).len()));
        }
    }

    #[test]
    fn heavy_mobility_keeps_the_most_moves_open() {
        let b = busy_board();
        let all = moves(&b, 1, 0.0);
        let most = all.iter().filter_map(|m| m.1.left).max();

        assert!(
            all.iter().any(|m| m.1.left != most),
            "the board must offer a choice"
        );

        assert_eq!(moves(&b, 1, 1000.0)[0].1.left, most);
    }
}
