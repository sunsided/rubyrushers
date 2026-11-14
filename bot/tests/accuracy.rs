//! How well the bot reads boards: the web build deals seeded boards, the bot reads them
//! from screenshots, and the game's own state (which the bot never sees) grades it.
//!
//! Needs Chromium and a web build: `trunk build` at the workspace root, then
//! `cargo test -p rubyrushers-bot --release -- --ignored --nocapture` (or
//! `task bot:accuracy`).

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use rubyrushers::logic::{Mode, Pos, Power, Session};
use rubyrushers_bot::browser::Screen;
use rubyrushers_bot::{Board, Grid, Tile, read, serve::serve};

/// The truth for cell (x, y) of a seeded endless game: colour, or `None` for a wild.
fn truth(session: &Session, x: usize, y: usize) -> Option<u8> {
    let g = session
        .board()
        .get(Pos::new(x as i32, y as i32))
        .expect("full board");

    (g.power != Power::Wild).then_some(g.color)
}

#[test]
#[ignore = "needs Chromium and a web build in dist/"]
fn reads_seeded_boards() {
    let dist = std::env::var_os("DIST")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../dist"));

    let base = serve(&dist).expect("serve dist/");
    let mut screen = Screen::open(&base, 1120, 840, false).expect("open Chromium");

    let mut worst = 1.0f32;
    for calm in [true, false] {
        worst = worst.min(accuracy(&mut screen, &base, calm));
    }

    assert!(worst >= 0.99, "accuracy {worst}");
}

/// The share of cells the bot reads right over eight seeded boards, with calm or full
/// effects. Fails if two of its classes stand for one colour.
fn accuracy(screen: &mut Screen, base: &str, calm: bool) -> f32 {
    let (mut right, mut total) = (0, 0);
    // Which of the bot's classes stands for which colour of the game's.
    let mut mapping: HashMap<u8, u8> = HashMap::new();

    for seed in 1..=8u64 {
        let board = read_seeded(screen, base, seed, calm);
        let session = Session::new(Mode::Endless, seed);

        for (y, row) in board.iter().enumerate() {
            for (x, &tile) in row.iter().enumerate() {
                total += 1;
                if graded(tile, truth(&session, x, y), &mut mapping) {
                    right += 1;
                } else {
                    println!("seed {seed} calm {calm}: ({x}, {y}) read {tile:?}");
                }
            }
        }
    }

    let mut targets: Vec<u8> = mapping.values().copied().collect();
    targets.sort_unstable();
    targets.dedup();

    assert_eq!(
        targets.len(),
        mapping.len(),
        "two classes read as one colour"
    );

    let accuracy = right as f32 / total as f32;
    println!(
        "calm {calm}: {right}/{total} cells read right ({:.2}%)",
        accuracy * 100.0
    );

    accuracy
}

/// Deals the endless board of `seed` and reads it from a screenshot.
fn read_seeded(screen: &mut Screen, base: &str, seed: u64, calm: bool) -> Board {
    let effects = if calm { "&calm" } else { "" };

    screen
        .goto(&format!("{base}?mode=endless&seed={seed}{effects}"))
        .unwrap();

    // Past the "Go!" banner, before the idle hint.
    std::thread::sleep(Duration::from_millis(2500));
    let img = screen.screenshot().unwrap();
    let grid = Grid::locate(&img).expect("board found");

    read::board(&img, &grid)
}

/// Whether `tile` as read matches the game's `truth`; the first time a class is seen, it
/// is taken to stand for the colour it was read on (`mapping`).
fn graded(tile: Tile, truth: Option<u8>, mapping: &mut HashMap<u8, u8>) -> bool {
    match (tile, truth) {
        (Tile::Gem(c), Some(t)) => *mapping.entry(c).or_insert(t) == t,
        (Tile::Wild, None) => true,
        _ => false,
    }
}
