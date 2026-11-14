//! Plays RubyRushers in Chromium from screenshots alone.
//!
//! ```text
//! rubyrushers-bot [--dist dist | --url http://localhost:8080/] [--mode rush|endless|tilt]
//!                 [--full] [--visible] [--watch | --watch-shot w.png] [--secs 90]
//!                 [--shot final.png] [--seed 7] [--mobility 6]
//!                 [--time cap|fade]
//! ```
//!
//! By default it serves the web build in `dist/` itself, starts a Rush game with calm
//! effects in a headless browser and plays until the game ends, as fast as it can: it
//! moves wherever two readings in a row agree, without waiting for the board to rest. The
//! game is over when the bot finds no move for a while or the board ignores its moves.
//!
//! In Endless, where a board without moves ends the game, it takes its time: it moves
//! only once the whole board reads still, and it values the moves a swap leaves open
//! (`--mobility` sets the weight; 0 turns it off).
//!
//! `--watch` opens a window that shows what the bot sees and thinks while it plays;
//! `--watch-shot <file.png>` does too, saves a picture of it once the bot is done, and
//! closes it.

use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use rubyrushers_bot::browser::Screen;
use rubyrushers_bot::insight::{self, Insight, Plan};
use rubyrushers_bot::{Board, Grid, SIZE, Tile, read, serve::serve, sim, solver, viewer};

/// Seconds without a move the bot can trust before it calls the game over: the board is
/// gone, covered, or too dimmed to read alike twice. While the game runs, a move turns up
/// well within a second.
const STUCK_SECS: f32 = 4.0;
/// Time between the two clicks of a swap: two frames at 60 Hz.
const CLICK_GAP: Duration = Duration::from_millis(34);
/// How long the cells a move changes stay off limits: swap, clear, fall.
const BUSY: Duration = Duration::from_millis(700);
/// Moves in a row that change nothing before the bot calls the game over: a finished game
/// may still show its board, dimmed under the final score, yet ignores every click.
const DEAD_MOVES: u32 = 3;
/// Candidate moves kept for the viewer.
const SHOWN_CANDIDATES: usize = 8;

/// Reads the flags and plays; with `--watch`, plays on a second thread while the viewer
/// window takes the main one.
fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let shot = args
        .iter()
        .position(|a| a == "--watch-shot")
        .and_then(|i| args.get(i + 1))
        .map(Into::into);
    if shot.is_none() && !args.iter().any(|a| a == "--watch") {
        return play(&args, false);
    }
    let bot = std::thread::spawn(move || {
        let result = play(&args, true);
        if let Err(e) = &result {
            eprintln!("error: {e:#}");
        }
        result
    });
    let shown = viewer::show(shot);
    // The window closed: stop the bot if it still plays.
    insight::STOP.store(true, Ordering::Relaxed);
    let played = bot.join().expect("the bot thread does not panic");
    shown?;
    played
}

/// Opens the game and plays it; `watch` publishes every round to [`insight::LATEST`].
fn play(args: &[String], watch: bool) -> Result<()> {
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
    let time = value("--time").map_or(String::new(), |t| format!("&time={t}"));
    let secs: f32 = value("--secs").map_or(Ok(600.0), |s| s.parse())?;
    let seed = value("--seed").map_or(String::new(), |s| format!("&seed={s}"));
    let patient = mode == "endless";
    // Only Endless ends when no move is left; elsewhere the board reshuffles.
    let mobility: f32 = match value("--mobility") {
        Some(w) => w.parse().context("--mobility needs a number")?,
        None if mode == "endless" => solver::MOBILITY,
        None => 0.0,
    };

    let mut screen = Screen::open(
        &format!("{base}?mode={mode}{calm}{time}{seed}"),
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
    let grid = locate(&mut screen)?;
    println!("board at {:?}, {:.1} px per cell", grid.origin, grid.pitch);
    // From now on only the board is captured; `view` is the grid within that crop.
    let side = grid.pitch * SIZE as f32;
    let corner = [
        grid.origin[0].max(0.0).floor(),
        grid.origin[1].max(0.0).floor(),
    ];
    screen.set_clip(Some([
        corner[0],
        corner[1],
        side.ceil() + 1.0,
        side.ceil() + 1.0,
    ]));
    let view = Grid {
        origin: [grid.origin[0] - corner[0], grid.origin[1] - corner[1]],
        pitch: grid.pitch,
    };

    let start = Instant::now();
    let mut last: Option<Board> = None;
    let mut moved = Instant::now();
    // Per cell: until when the bot's own last moves are expected to keep it changing.
    let mut busy = [[start; SIZE]; SIZE];
    let mut moves = 0;
    // Moves made, waiting to be checked once settled: when, and the board read just before.
    let mut pending: Vec<(Instant, Board)> = Vec::new();
    let mut dead = 0;
    // Seconds spent capturing, reading, solving and clicking; and loop rounds.
    let mut spent = [0.0f32; 4];
    let mut rounds = 0u32;
    let per_round =
        |spent: &[f32; 4], rounds: u32| spent.map(|s| s * 1000.0 / rounds.max(1) as f32);
    while start.elapsed().as_secs_f32() < secs && !insight::STOP.load(Ordering::Relaxed) {
        rounds += 1;
        let t0 = Instant::now();
        let img = screen.screenshot()?;
        let t1 = Instant::now();
        let board = read::board(&img, &view);
        let now = Instant::now();
        spent[0] += (t1 - t0).as_secs_f32();
        spent[1] += (now - t1).as_secs_f32();
        if moved.elapsed().as_secs_f32() > STUCK_SECS {
            println!("no move in {STUCK_SECS} s: game over");
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
        // While the game runs, a move changes the board, or something else does meanwhile.
        // A board that reads exactly the same after the move has settled ignored it.
        let mut over = false;
        pending.retain(|(due, before)| {
            if now < *due {
                return true;
            }
            dead = if board == *before { dead + 1 } else { 0 };
            over |= dead >= DEAD_MOVES;
            false
        });
        if over {
            println!("the board ignores moves: game over");
            break;
        }
        // A dimmed or covered board (game over, banners) is no place to click.
        let seen = board
            .iter()
            .flatten()
            .filter(|t| **t != Tile::Unknown)
            .count();
        // Without a clock there is no hurry: wait until every cell is trusted, so the
        // prediction (and the moves it leaves open) covers the whole board.
        let settled = trusted.iter().flatten().all(|t| *t != Tile::Unknown);
        let clear = seen * 4 >= SIZE * SIZE * 3 && (!patient || settled);
        let t2 = Instant::now();
        let ranked = if clear {
            solver::moves(&trusted, solver::LOOKAHEAD, mobility)
        } else {
            Vec::new()
        };
        let t3 = Instant::now();
        spent[2] += (t3 - t2).as_secs_f32();

        let mut plan = None;
        if let Some(&(swap @ (a, b), value)) = ranked.first() {
            screen.click(grid.center(a[0], a[1]))?;
            // The game reads one press per frame: two clicks in one frame would be one.
            std::thread::sleep(CLICK_GAP);
            screen.click(grid.center(b[0], b[1]))?;
            spent[3] += t3.elapsed().as_secs_f32();
            moves += 1;
            moved = Instant::now();
            println!("move {moves}: {a:?} <-> {b:?}");
            // Leave the cells this move will change alone until they settled.
            let after = sim::play(&trusted, swap).map_or(trusted, |o| o.after);
            let until = Instant::now() + BUSY;
            pending.push((until, board));
            for y in 0..SIZE {
                for x in 0..SIZE {
                    if after[y][x] != trusted[y][x] || [x, y] == a || [x, y] == b {
                        busy[y][x] = until;
                    }
                }
            }
            if watch {
                let next = solver::moves(&after, 0, 0.0).first().copied();
                plan = Some(Plan {
                    before: trusted,
                    candidates: ranked.iter().take(SHOWN_CANDIDATES).copied().collect(),
                    swap,
                    value,
                    after,
                    next,
                });
            }
        }

        if watch {
            let status = if plan.is_some() {
                "moving"
            } else if patient && !settled {
                "waiting for the board to settle"
            } else if !clear {
                "board covered: waiting"
            } else {
                "no trusted move: waiting"
            };
            *insight::LATEST.lock().expect("the viewer does not panic") = Some(Insight {
                crop: img,
                view,
                read: board,
                trusted,
                busy: busy.map(|row| row.map(|until| now < until)),
                plan,
                moves,
                elapsed: start.elapsed().as_secs_f32(),
                timings: per_round(&spent, rounds),
                status: status.into(),
            });
        }
    }
    println!("{moves} moves in {:.0} s", start.elapsed().as_secs_f32());
    let ms = per_round(&spent, rounds);
    println!(
        "{rounds} rounds, per round: capture {:.0} ms, read {:.1} ms, solve {:.1} ms, click {:.0} ms",
        ms[0], ms[1], ms[2], ms[3],
    );
    insight::FINISHED.store(true, Ordering::Relaxed);
    if let Some(path) = value("--shot") {
        screen.set_clip(None);
        std::fs::write(&path, screen.png()?)?;
        println!("final screen saved to {path}");
    }
    Ok(())
}

/// Finds the board, retrying while effects still cover it.
fn locate(screen: &mut Screen) -> Result<Grid> {
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
