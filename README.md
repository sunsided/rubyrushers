# RubyRushers

A fast match-three gem game built on [game-toolkit](https://github.com/sunsided/game-toolkit).
Swap neighbouring gems to line up three or more; bigger lines leave special gems behind.

- **Rush**: 60 seconds on the clock, time gems add more. Stuck boards are reshuffled.
- **Endless**: no clock; the game ends when no move is left.

Special gems:

| Match | Earns | When cleared |
|-------|-------|--------------|
| 4 in a line | Bomb (glowing ring) | clears the 3×3 around it |
| L or T shape | Cross (white plus) | clears its row and column |
| 5 in a line | Wild (rainbow orb) | swap it with a gem to clear that colour; two wilds clear the board |

Chain reactions score a cascade multiplier.

## Run

```sh
task run                      # or: cargo run --release
task run -- --mode rush       # skip the title
task screenshot MODE=endless  # save a frame to screenshots/window.png
```

Click a gem and then a neighbour, or drag a gem onto a neighbour. F12 saves a screenshot,
Esc goes back to the title. Linux needs `libasound2-dev libdbus-1-dev libudev-dev pkg-config`.

## Roadmap

1. The game: playable and fun. ← here
2. A screen-capture autoplayer that sees only pixels, successor to
   [rubyrush](https://github.com/sunsided/rubyrush).

## License

[EUPL-1.2](LICENSE).
