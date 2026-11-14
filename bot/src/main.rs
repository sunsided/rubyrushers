//! Plays RubyRushers in Chromium from screenshots alone.
//!
//! ```text
//! rubyrushers-bot [--dist dist | --url http://localhost:8080/] [--mode rush|endless|tilt]
//!                 [--full] [--visible] [--secs 90] [--shot final.png]
//! ```
//!
//! By default it serves the web build in `dist/` itself, starts a Rush game with calm
//! effects in a headless browser and plays until the game ends, as fast as it can: it
//! moves wherever two readings in a row agree, without waiting for the board to rest.

use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use rubyrushers_bot::browser::Screen;
use rubyrushers_bot::{Board, Grid, SIZE, Tile, read, serve::serve, sim, solver};

/// Seconds the board may stay unreadable before the bot calls the game over.
const GONE_SECS: f32 = 4.0;
/// How long the cells a move changes stay off limits: swap, clear, fall.
const BUSY: Duration = Duration::from_millis(700);

/// Reads the flags, opens the game and plays it.
fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let value = |flag: &str| {
        args.iter()
            .position(|a| a == flag)
            .and_then(|i| args.get(i + 1).cloned())
    };
    let flag = |f: &str| args.iter().any(|a| a == f);

    let base = match value("--url") {
        Some(url) => url,
        None => serve(value("--dist").unwrap_or_else(|| "dist".into()))
            .context("serve the web build (run `trunk build` first)")?,
    };
    let mode = value("--mode").unwrap_or_else(|| "rush".into());
    if !["rush", "endless", "tilt"].contains(&mode.as_str()) {
        bail!("--mode needs rush, endless or tilt");
    }
    let calm = if flag("--full") { "" } else { "&calm" };
    let secs: f32 = value("--secs").map_or(Ok(600.0), |s| s.parse())?;

    let screen = Screen::open(
        &format!("{base}?mode={mode}{calm}"),
        1120,
        840,
        flag("--visible"),
    )?;
    let fps = screen.fps()?;
    println!("the page renders at {fps:.0} fps");
    if fps < 30.0 {
        println!("warning: the game runs slow in this browser; try --visible");
    }
    // Past the "Go!" banner.
    std::thread::sleep(Duration::from_millis(1500));
    let grid = locate(&screen)?;
    println!("board at {:?}, {:.1} px per cell", grid.origin, grid.pitch);

    let start = Instant::now();
    let mut last: Option<Board> = None;
    let mut readable = Instant::now();
    // Per cell: until when the bot's own last moves are expected to keep it changing.
    let mut busy = [[start; SIZE]; SIZE];
    let mut moves = 0;
    while start.elapsed().as_secs_f32() < secs {
        let img = screen.screenshot()?;
        let board = read::board(&img, &grid);
        let now = Instant::now();
        let seen = board
            .iter()
            .flatten()
            .filter(|t| **t != Tile::Unknown)
            .count();
        if seen * 2 > SIZE * SIZE {
            readable = now;
        } else if readable.elapsed().as_secs_f32() > GONE_SECS {
            println!("the board is gone: game over");
            break;
        }
        // Trust a cell when two readings in a row agree and no move of the bot's is still
        // working on it; everything else is unknown to the solver. No waiting for the
        // whole board to rest: any move among trusted cells goes.
        let mut trusted = [[Tile::Unknown; SIZE]; SIZE];
        if let Some(prev) = last {
            for y in 0..SIZE {
                for x in 0..SIZE {
                    if board[y][x] == prev[y][x] && now >= busy[y][x] {
                        trusted[y][x] = board[y][x];
                    }
                }
            }
        }
        last = Some(board);
        let Some((a, b)) = solver::best(&trusted) else {
            continue;
        };
        screen.click(grid.center(a[0], a[1]))?;
        screen.click(grid.center(b[0], b[1]))?;
        moves += 1;
        println!("move {moves}: {a:?} <-> {b:?}");
        // Leave the cells this move will change alone until they settled.
        let after = sim::play(&trusted, (a, b)).map_or(trusted, |o| o.after);
        let until = Instant::now() + BUSY;
        for y in 0..SIZE {
            for x in 0..SIZE {
                if after[y][x] != trusted[y][x] || [x, y] == a || [x, y] == b {
                    busy[y][x] = until;
                }
            }
        }
    }
    println!("{moves} moves in {:.0} s", start.elapsed().as_secs_f32());
    if let Some(path) = value("--shot") {
        std::fs::write(&path, screen.png()?)?;
        println!("final screen saved to {path}");
    }
    Ok(())
}

/// Finds the board, retrying while effects still cover it.
fn locate(screen: &Screen) -> Result<Grid> {
    let mut error = None;
    for _ in 0..50 {
        match Grid::locate(&screen.screenshot()?) {
            Ok(g) => return Ok(g),
            Err(e) => error = Some(e),
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    Err(error.expect("tried at least once")).context("no board on screen")
}
