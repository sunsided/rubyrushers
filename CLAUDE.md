# RubyRushers - agent notes

A match-three game on game-toolkit 0.11. The toolkit API cheatsheet is
`../game-toolkit/templates/jam-agent-guide.md` (or docs.rs/game-toolkit); read it instead of
the toolkit sources.

## Layout

- `game/src/logic/` - the rules, toolkit-free and deterministic per seed. The board runs in
  real time: `Session::update(dt)` advances `Field` (each gem's cell plus its `Motion`:
  swap tween, fall along the current `Gravity`), matches only settled gems and reports
  `Event`s. Gravity works on lanes and depths, so every direction shares one code path;
  it only turns while the board is still. The player may swap
  any two settled gems while the rest still moves.
- `game/src/render/` - draws the session's pieces in a 960x720 design space scaled to the
  window (`Layout`). `Fx` turns events into vanishing gems, sparks and popups; `Effects`
  (Full/Calm) gates shake, spin, pulsing, the board's glowing frame, halos, shine and
  landing squash, and ambient sparkle; `showcase` draws the title backdrop; `gem_art` rasterises the
  gem atlas at start-up.
- `game/src/sound.rs` - synthesised effects (`synth` feature), no audio files.
- `game/src/app.rs` - the only glue between input, logic, view and sound.
- `bot/` - the autoplayer (workspace member). It must never depend on the game crate
  outside `[dev-dependencies]`: only its accuracy harness (`bot/tests/accuracy.rs`) may
  look at the game's state, to grade what the bot read from pixels. `--watch` plays on a
  worker thread and shows each round (`insight::Insight`) in a game-toolkit window
  (`viewer.rs`); the toolkit is a normal dependency, the game crate is not.

## Conventions

- One type per module (`name.rs` + `name/`, never `mod.rs`); doc comments on everything,
  private items included.
- Every rule change gets a test in `logic/`. `task check` (fmt, clippy -D warnings, tests,
  docs) must pass.
- Own names and look: no other game's names in identifiers, text or assets.
- Keep gems distinct by silhouette and colour: the bot reads only screenshots. Do not add
  hooks that expose game state to it. Anything drawn between gems must stay dark and
  unsaturated (the bot finds gems as colourful blobs): rerun `task bot:accuracy` after
  visual changes. The web build takes the CLI flags as query
  parameters (`?mode=endless&seed=3&calm`); that is how the harness deals known boards.
- Keep code wasm-safe (no `std::time::Instant`; `SystemTime` only behind `cfg`).
