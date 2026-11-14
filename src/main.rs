//! Launcher. Everything lives in the library crate; see [`rubyrushers::app`].
//!
//! Command line (native), or the same as URL query parameters on the web
//! (`?mode=tilt&seed=7&calm`):
//!
//! - `--mode rush|endless|tilt`: skip the title and start that mode.
//! - `--seed <n>`: deal the first game from this seed.
//! - `--calm`: start with calm effects (no shake, spin or glints; F2 toggles).
//! - `--screenshot <file.png>`: save what the window shows after `--at <secs>` seconds
//!   (default 3), then quit.

use game_toolkit::prelude::*;
use rubyrushers::app::{App, Options, config, set_options};
use rubyrushers::logic::Mode;

/// Parses the command line, sets up logging (native only) and hands the game to the
/// toolkit's run loop.
fn main() -> Result<()> {
    let args = arguments();
    let value = |flag: &str| {
        args.iter()
            .position(|a| a == flag)
            .map(|i| args.get(i + 1).cloned().unwrap_or_default())
    };
    let fail = |msg: &str| -> ! {
        eprintln!("{msg}");
        std::process::exit(2)
    };
    let mode = value("--mode").map(|m| match m.as_str() {
        "rush" => Mode::rush(),
        "endless" => Mode::Endless,
        "tilt" => Mode::tilt(),
        _ => fail("--mode needs rush, endless or tilt"),
    });
    let seed = value("--seed").map(|s| s.parse().unwrap_or_else(|_| fail("--seed needs a number")));
    let at = value("--at").map_or(3.0, |s| {
        s.parse().unwrap_or_else(|_| fail("--at needs seconds"))
    });
    let screenshot = value("--screenshot").map(|p| {
        if p.is_empty() {
            fail("--screenshot needs a file name");
        }
        (p.into(), at)
    });
    set_options(Options {
        mode,
        seed,
        screenshot,
        calm: args.iter().any(|a| a == "--calm"),
    });
    #[cfg(not(target_arch = "wasm32"))]
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    run::<App>(config())
}

/// The command line natively; on the web, the page's query string turned into the same
/// flags (`?mode=tilt&calm` becomes `--mode tilt --calm`).
fn arguments() -> Vec<String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::env::args().skip(1).collect()
    }
    #[cfg(target_arch = "wasm32")]
    {
        let query = web_sys::window()
            .and_then(|w| w.location().search().ok())
            .unwrap_or_default();
        query
            .trim_start_matches('?')
            .split('&')
            .filter(|kv| !kv.is_empty())
            .flat_map(|kv| match kv.split_once('=') {
                Some((k, v)) => vec![format!("--{k}"), v.to_owned()],
                None => vec![format!("--{kv}")],
            })
            .collect()
    }
}
