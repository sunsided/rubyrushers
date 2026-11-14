# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Rush (60 seconds, time gems add more), Endless (until no move is left) and Tilt (gravity
  turns a quarter every 15 seconds), with special gems, cascades and speed bonuses.
- Calm and full effect levels; the browser picks calm under `prefers-reduced-motion`.
- A browser build, on GitHub Pages and on games.markusmayer.dev.
- The bot (`bot/`), which plays from screenshots alone and shows what it sees and thinks.
- Releases are cut with `task release` (cargo-release); a version tag builds the Linux
  binary and the web zip, attests both, publishes the release and announces it to
  games.markusmayer.dev.

[Unreleased]: https://github.com/sunsided/rubyrushers/commits/main
