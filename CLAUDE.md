# RubyRushers - agent notes

A match-three game on game-toolkit 0.11. The toolkit API cheatsheet is
`../game-toolkit/templates/jam-agent-guide.md` (or docs.rs/game-toolkit); read it instead of
the toolkit sources.

## Layout

- `src/logic/` - the rules, toolkit-free and deterministic per seed. A move resolves
  instantly in `Session::swap` into `Step`s (swap, clear, settle, reshuffle).
- `src/render/` - draws in a 960x720 design space scaled to the window (`Layout`).
  `BoardView` plays the `Step`s back as animations and emits `Cue`s; `Fx` reacts with
  sparks and popups; `gem_art` rasterises the gem atlas at start-up.
- `src/sound.rs` - synthesised effects (`synth` feature), no audio files.
- `src/app.rs` - the only glue between input, logic, view and sound.

## Conventions

- One type per module (`name.rs` + `name/`, never `mod.rs`); doc comments on everything,
  private items included.
- Every rule change gets a test in `logic/`. `task check` (fmt, clippy -D warnings, tests,
  docs) must pass.
- Own names and look: no other game's names in identifiers, text or assets.
- Keep gems distinct by silhouette and colour, and the board geometry stable: phase 2 is a
  bot that reads only screenshots. Do not add hooks that expose game state to it.
- Keep code wasm-safe (no `std::time::Instant`; `SystemTime` only behind `cfg`).
