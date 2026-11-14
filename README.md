# RubyRushers

A fast match-three gem game built on [game-toolkit](https://github.com/sunsided/game-toolkit).
Swap neighbouring gems to line up three or more; bigger lines leave special gems behind.

- **Rush**: 60 seconds on the clock, time gems add more. Stuck boards are reshuffled.
- **Endless**: no clock; the game ends when no move is left.
- **Tilt**: like Rush, but gravity turns a quarter clockwise every 15 seconds (down, left,
  up, right). Arrows warn just before; the board settles, then gems fall the new way.

Special gems:

| Match | Earns | When cleared |
|-------|-------|--------------|
| 4 in a line | Bomb (glowing ring) | clears the 3×3 around it |
| L or T shape | Cross (white plus) | clears its row and column |
| 5 in a line | Wild (rainbow orb) | swap it with a gem to clear that colour; two wilds clear the board |

Chain reactions score a cascade multiplier, and matches in quick succession (within
2.5 s) build a speed streak worth +50% per level. Best scores are kept per mode.

## Run

```sh
task run                      # or: cargo run --release
task run -- --mode tilt       # skip the title (rush, endless, tilt)
task screenshot MODE=endless  # save a frame to screenshots/window.png
task web                      # in the browser at http://localhost:8080
```

Click a gem and then a neighbour, or drag a gem onto a neighbour. Keep going while
cascades still fall: any gem at rest can be swapped. F2 switches the effects between
Full and Calm (no shake, spin, glints or pulsing; picked automatically when the browser
asks for reduced motion, or with `--calm`). F12 saves a screenshot, Esc goes back to the
title. Linux needs `libasound2-dev libdbus-1-dev libudev-dev pkg-config`.

## Roadmap

1. The game: playable and fun. ← here
2. A screen-capture autoplayer that sees only pixels, successor to
   [rubyrush](https://github.com/sunsided/rubyrush).

## License

[EUPL-1.2](LICENSE).
