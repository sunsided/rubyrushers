//! Special gems going off: what each blast takes out, and the blasts it sets off.

use std::collections::BTreeSet;

use super::board::{Board, SIZE};
use super::gem::COLORS;
use super::pos::Pos;
use super::power::Power;
use super::rng::Rng;

/// Sets off every special gem among `doomed`, and those their blasts reach, adding the
/// cells they take out. Returns each blast with the cells it took out.
pub fn detonate(
    board: &Board,
    rng: &mut Rng,
    doomed: &mut BTreeSet<Pos>,
    wild_colors: &[(Pos, u8)],
) -> Vec<(Power, Pos, Vec<Pos>)> {
    let mut blasts = Vec::new();
    let mut done: BTreeSet<Pos> = BTreeSet::new();

    loop {
        let next = doomed
            .iter()
            .copied()
            .find(|p| !done.contains(p) && board.get(*p).is_some_and(|g| g.power != Power::None));
        let Some(at) = next else {
            break;
        };

        done.insert(at);
        let power = board.get(at).map(|g| g.power).unwrap_or_default();
        let reach = reach(board, rng, at, power, wild_colors);

        doomed.extend(reach.iter().copied());
        blasts.push((power, at, reach));
    }

    blasts
}

/// The cells the `power` gem at `at` takes out. A wild takes out its colour from
/// `wild_colors`, or a random colour still on the board.
fn reach(
    board: &Board,
    rng: &mut Rng,
    at: Pos,
    power: Power,
    wild_colors: &[(Pos, u8)],
) -> Vec<Pos> {
    match power {
        Power::None => Vec::new(),
        Power::Bomb => (-1..=1)
            .flat_map(|dy| (-1..=1).map(move |dx| Pos::new(at.x + dx, at.y + dy)))
            .filter(|p| p.on_board())
            .collect(),
        Power::Cross => (0..SIZE)
            .flat_map(|i| [Pos::new(i, at.y), Pos::new(at.x, i)])
            .collect(),
        Power::Wild => {
            let color = wild_colors
                .iter()
                .find(|(p, _)| *p == at)
                .map(|&(_, c)| c)
                .unwrap_or_else(|| random_present_color(board, rng));

            Pos::all()
                .filter(|p| is_plain_of(board, *p, color))
                .collect()
        }
    }
}

/// Whether the gem at `p` has `color` and is not itself a wild.
fn is_plain_of(board: &Board, p: Pos, color: u8) -> bool {
    board
        .get(p)
        .is_some_and(|g| g.power != Power::Wild && g.color == color)
}

/// A colour some non-wild gem on the board has (any colour if none has).
fn random_present_color(board: &Board, rng: &mut Rng) -> u8 {
    let present: Vec<u8> = (0..COLORS)
        .filter(|&c| Pos::all().any(|p| is_plain_of(board, p, c)))
        .collect();

    if present.is_empty() {
        rng.below(u32::from(COLORS)) as u8
    } else {
        present[rng.below(present.len() as u32) as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bombs_and_crosses_chain() {
        // The bomb at (1, 0) reaches the cross at (1, 1), which takes out row 1 and
        // column 1.
        let board = Board::parse(&["0A0", "1b3", "234"]);
        let mut doomed: BTreeSet<Pos> = [Pos::new(1, 0)].into();
        let blasts = detonate(&board, &mut Rng::new(1), &mut doomed, &[]);
        let powers: Vec<Power> = blasts.iter().map(|b| b.0).collect();
        assert_eq!(powers, vec![Power::Bomb, Power::Cross]);
        assert!((0..SIZE).all(|i| doomed.contains(&Pos::new(i, 1))));
        assert!((0..SIZE).all(|i| doomed.contains(&Pos::new(1, i))));
    }

    #[test]
    fn a_wild_with_a_colour_takes_that_colour() {
        let board = Board::parse(&["W30", "313", "033"]);
        let wild = Pos::new(0, 0);
        let mut doomed: BTreeSet<Pos> = [wild].into();
        detonate(&board, &mut Rng::new(1), &mut doomed, &[(wild, 3)]);
        let threes = Pos::all()
            .filter(|p| {
                board
                    .get(*p)
                    .is_some_and(|g| g.color == 3 && g.power == Power::None)
            })
            .count();
        assert_eq!(doomed.len(), threes + 1);
    }

    #[test]
    fn a_wild_without_a_colour_picks_one_on_the_board() {
        let board = Board::parse(&["W5", "55"]);
        let mut doomed: BTreeSet<Pos> = [Pos::new(0, 0)].into();
        detonate(&board, &mut Rng::new(9), &mut doomed, &[]);
        assert_eq!(doomed.len(), 4);
    }
}
