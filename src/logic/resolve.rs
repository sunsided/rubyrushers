//! Playing out a move: matches, special gems going off, gravity, refills, cascades.

use std::collections::BTreeSet;

use super::board::{Board, SIZE};
use super::gem::COLORS;
use super::matcher;
use super::pos::Pos;
use super::power::Power;
use super::rng::Rng;
use super::score;
use super::step::{Clear, Step};

/// What starts a resolution besides matches on the board.
#[derive(Clone, Debug, Default)]
pub struct Trigger {
    /// The cells the player swapped (reward gems appear there).
    pub moved: Vec<Pos>,
    /// Cells cleared no matter what (a wild swap).
    pub forced: Vec<Pos>,
    /// Wild gems set off with a chosen colour instead of a random one.
    pub wild_colors: Vec<(Pos, u8)>,
}

/// Resolves the board until it is still: each round clears every match (and whatever
/// special gems take with them), lets gems fall and refills. One in `time_odds` new gems
/// carries a time bonus.
pub fn resolve(board: &mut Board, rng: &mut Rng, trigger: Trigger, time_odds: u32) -> Vec<Step> {
    let mut steps = Vec::new();
    let mut trigger = trigger;
    for cascade in 1.. {
        let groups = matcher::find(board, &trigger.moved);
        if groups.is_empty() && trigger.forced.is_empty() {
            break;
        }
        let mut doomed: BTreeSet<Pos> = groups
            .iter()
            .flat_map(|g| g.cells.iter().copied())
            .chain(trigger.forced.iter().copied())
            .collect();
        let blasts = detonate(board, rng, &mut doomed, &trigger.wild_colors);

        let cleared: Vec<_> = doomed
            .iter()
            .filter_map(|&p| board.get(p).map(|g| (g, p)))
            .collect();
        for &p in &doomed {
            board.set(p, None);
        }
        let mut created = Vec::new();
        for g in groups.iter().filter(|g| g.reward != Power::None) {
            let color = board_color(&cleared, g.spawn);
            let gem = board.new_gem(color, g.reward, false);
            board.set(g.spawn, Some(gem));
            created.push((gem, g.spawn));
        }

        let rewards: Vec<Power> = groups.iter().map(|g| g.reward).collect();
        steps.push(Step::Clear(Clear {
            cascade,
            points: score::points(cleared.len(), &rewards, blasts.len(), cascade),
            time_gems: cleared.iter().filter(|(g, _)| g.time_bonus).count() as u32,
            cleared,
            blasts,
            created,
        }));
        let falls = board.collapse();
        let spawns = board.refill(rng, time_odds);
        steps.push(Step::Settle { falls, spawns });
        trigger = Trigger::default();
    }
    steps
}

/// The colour of the gem cleared at `p` (the reward gem takes it over).
fn board_color(cleared: &[(super::gem::Gem, Pos)], p: Pos) -> u8 {
    cleared
        .iter()
        .find(|(_, q)| *q == p)
        .map_or(0, |(g, _)| g.color)
}

/// Sets off every special gem among `doomed`, and those their blasts reach, adding the
/// cells they take out. Returns each blast with the cells it took out.
fn detonate(
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
        let reach: Vec<Pos> = match power {
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
                    .filter(|p| {
                        board
                            .get(*p)
                            .is_some_and(|g| g.power != Power::Wild && g.color == color)
                    })
                    .collect()
            }
        };
        doomed.extend(reach.iter().copied());
        blasts.push((power, at, reach));
    }
    blasts
}

/// A colour some non-wild gem on the board has (any colour if none has).
fn random_present_color(board: &Board, rng: &mut Rng) -> u8 {
    let present: Vec<u8> = (0..COLORS)
        .filter(|&c| {
            Pos::all().any(|p| {
                board
                    .get(p)
                    .is_some_and(|g| g.power != Power::Wild && g.color == c)
            })
        })
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

    /// The clears of a resolution.
    fn clears(steps: &[Step]) -> Vec<&Clear> {
        steps
            .iter()
            .filter_map(|s| match s {
                Step::Clear(c) => Some(c),
                _ => None,
            })
            .collect()
    }

    /// A full board without matches whose rows 0..rows.len() are replaced by `rows`.
    fn board_with(rows: &[&str]) -> Board {
        // Background: a dead pattern with no matches.
        let mut all: Vec<String> = (0..SIZE)
            .map(|y| {
                (0..SIZE)
                    .map(|x| char::from(b'0' + ((x + y) % 7) as u8))
                    .collect()
            })
            .collect();
        for (y, row) in rows.iter().enumerate() {
            let mut cells: Vec<char> = all[y].chars().collect();
            for (x, ch) in row.chars().enumerate() {
                if ch != ' ' {
                    cells[x] = ch;
                }
            }
            all[y] = cells.into_iter().collect();
        }
        let refs: Vec<&str> = all.iter().map(String::as_str).collect();
        Board::parse(&refs)
    }

    #[test]
    fn a_still_board_resolves_to_nothing() {
        let mut board = board_with(&[]);
        assert!(resolve(&mut board, &mut Rng::new(1), Trigger::default(), 0).is_empty());
    }

    #[test]
    fn a_match_clears_falls_and_refills() {
        let mut board = board_with(&["000"]);
        let steps = resolve(&mut board, &mut Rng::new(1), Trigger::default(), 0);
        let first = clears(&steps)[0];
        assert_eq!(first.cascade, 1);
        assert_eq!(first.cleared.len(), 3);
        assert_eq!(first.points, 30);
        assert!(matches!(steps[1], Step::Settle { .. }));
        assert!(Pos::all().all(|p| board.get(p).is_some()));
        assert!(matcher::find(&board, &[]).is_empty());
    }

    #[test]
    fn a_match_of_four_leaves_a_bomb() {
        let mut board = board_with(&["", "", "", "", "", "", "", "5555"]);
        let moved = vec![Pos::new(1, SIZE - 1)];
        let trigger = Trigger {
            moved,
            ..Trigger::default()
        };
        let steps = resolve(&mut board, &mut Rng::new(1), trigger, 0);
        let first = clears(&steps)[0];
        assert_eq!(first.created.len(), 1);
        let (gem, at) = first.created[0];
        assert_eq!(
            (gem.power, gem.color, at),
            (Power::Bomb, 5, Pos::new(1, SIZE - 1))
        );
        // On the bottom row it does not fall.
        assert_eq!(board.get(at).map(|g| g.id), Some(gem.id));
    }

    #[test]
    fn bombs_and_crosses_chain() {
        // The match at the top sets off the bomb, which reaches the cross below it.
        let mut board = board_with(&["0A0", " b "]);
        let steps = resolve(&mut board, &mut Rng::new(1), Trigger::default(), 0);
        let first = clears(&steps)[0];
        let powers: Vec<Power> = first.blasts.iter().map(|b| b.0).collect();
        assert_eq!(powers, vec![Power::Bomb, Power::Cross]);
        // Row 0 (3 gems), the bomb's reach, then row 1 and column 1 in full.
        let cells: BTreeSet<Pos> = first.cleared.iter().map(|(_, p)| *p).collect();
        assert!((0..SIZE).all(|i| cells.contains(&Pos::new(i, 1))));
        assert!((0..SIZE).all(|i| cells.contains(&Pos::new(1, i))));
    }

    #[test]
    fn a_wild_with_a_colour_clears_that_colour() {
        let mut board = board_with(&["W"]);
        let wild = Pos::new(0, 0);
        let trigger = Trigger {
            forced: vec![wild],
            wild_colors: vec![(wild, 3)],
            ..Trigger::default()
        };
        let steps = resolve(&mut board, &mut Rng::new(1), trigger, 0);
        let first = clears(&steps)[0];
        assert!(
            first
                .cleared
                .iter()
                .all(|(g, _)| g.color == 3 || g.power == Power::Wild)
        );
        // Every colour-3 gem of the pattern: one per anti-diagonal step of 7.
        let threes = Pos::all().filter(|p| (p.x + p.y) % 7 == 3).count();
        assert_eq!(first.cleared.len(), threes + 1);
    }

    #[test]
    fn same_seed_same_result() {
        let run = |seed| {
            let mut board = board_with(&["000", "a11"]);
            resolve(&mut board, &mut Rng::new(seed), Trigger::default(), 5)
        };
        assert_eq!(run(7), run(7));
    }
}
