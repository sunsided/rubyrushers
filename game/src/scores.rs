//! Best scores per mode, kept between runs: a small text file natively, the browser's
//! local storage on the web.

use std::fmt::Write as _;

use crate::logic::Mode;

/// The key per mode in the saved text (title button order).
const KEYS: [&str; 3] = ["rush", "endless", "tilt"];

/// Best score per mode, in title button order.
pub type Bests = [u64; 3];

/// Reads `key=value` lines; unknown keys and bad lines are skipped.
#[must_use]
pub fn parse(text: &str) -> Bests {
    let mut bests = [0; 3];

    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };

        if let (Some(i), Ok(v)) = (
            KEYS.iter().position(|k| *k == key.trim()),
            value.trim().parse(),
        ) {
            bests[i] = v;
        }
    }

    bests
}

/// Writes one `key=value` line per mode.
#[must_use]
pub fn format(bests: &Bests) -> String {
    KEYS.iter()
        .zip(bests)
        .fold(String::new(), |mut out, (k, v)| {
            // Writing to a String cannot fail.
            let _ = writeln!(out, "{k}={v}");
            out
        })
}

/// The index of `mode` in [`Bests`].
#[must_use]
pub fn index(mode: &Mode) -> usize {
    match mode {
        Mode::Rush { .. } => 0,
        Mode::Endless => 1,
        Mode::Tilt { .. } => 2,
    }
}

/// The saved best scores (all zero when none were saved or they cannot be read).
#[must_use]
pub fn load() -> Bests {
    store::read().map(|t| parse(&t)).unwrap_or_default()
}

/// Saves the best scores; failures are logged, the game goes on.
pub fn save(bests: &Bests) {
    if let Err(e) = store::write(&format(bests)) {
        log::warn!("could not save best scores: {e}");
    }
}

/// Native: a file in the user's data directory.
#[cfg(not(target_arch = "wasm32"))]
mod store {
    use std::path::PathBuf;

    /// `$XDG_DATA_HOME/rubyrushers/best.txt`, else under `~/.local/share` or `%APPDATA%`.
    fn path() -> Option<PathBuf> {
        let env = |k| {
            std::env::var_os(k)
                .filter(|v| !v.is_empty())
                .map(PathBuf::from)
        };
        let base = env("XDG_DATA_HOME")
            .or_else(|| env("HOME").map(|h| h.join(".local/share")))
            .or_else(|| env("APPDATA"))?;
        Some(base.join("rubyrushers").join("best.txt"))
    }

    /// The saved text, if any.
    pub fn read() -> Option<String> {
        std::fs::read_to_string(path()?).ok()
    }

    /// Replaces the saved text.
    pub fn write(text: &str) -> std::io::Result<()> {
        let path = path().ok_or_else(|| std::io::Error::other("no data directory"))?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }

        std::fs::write(path, text)
    }
}

/// Web: the page's local storage.
#[cfg(target_arch = "wasm32")]
mod store {
    /// The local storage key.
    const KEY: &str = "rubyrushers.best";

    /// The page's local storage, if the browser allows it.
    fn storage() -> Option<web_sys::Storage> {
        web_sys::window()?.local_storage().ok().flatten()
    }

    /// The saved text, if any.
    pub fn read() -> Option<String> {
        storage()?.get_item(KEY).ok().flatten()
    }

    /// Replaces the saved text.
    pub fn write(text: &str) -> Result<(), String> {
        let storage = storage().ok_or("no local storage")?;
        storage.set_item(KEY, text).map_err(|e| format!("{e:?}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bests_round_trip_and_survive_junk() {
        let bests = [1200, 0, 345];
        assert_eq!(parse(&format(&bests)), bests);
        assert_eq!(
            parse("tilt=7\nnonsense\nrush=abc\nfoo=3\n endless = 9 "),
            [0, 9, 7]
        );
    }

    #[test]
    fn every_mode_has_its_own_slot() {
        let modes = [Mode::rush(), Mode::Endless, Mode::tilt()];
        let mut slots: Vec<usize> = modes.iter().map(index).collect();
        slots.dedup();
        assert_eq!(slots, vec![0, 1, 2]);
    }
}
