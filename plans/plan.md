# Demo Polish Plan

## Goals
- Make the Vello counter demo launchable with a single command and zero manual asset wrangling.
- Ship a small dist bundle (host + component + docs) that we can hand to stakeholders.
- Lock down the interface so regressions in input, redraw scheduling, or error handling are caught automatically.

## Remaining Work

### Build & Packaging
- Add a `just demo` recipe that builds the counter component in release mode, runs `cargo run -p frontier-wasm-host` with the release artifact, and surfaces obvious failures early.
- Add a `just dist` (or `scripts/build-dist.sh`) command that copies the release host binary, release counter component, and a short README into `dist/`, then zips the folder for sharing.
- Detect whether `cargo-component` is installed before invoking it; emit a friendly hint or install command in the Just recipes and in CI scripts.

### Demo Experience
- Allow `frontier-wasm-host` to fall back to an embedded counter component when `--component` is omitted, so the demo works out of the box.
- Extend the README with a 30-second "run the demo" section (commands, expected window, controls, restart shortcut) and drop in a fresh screenshot/gif.
- Make the error overlay more actionable: add the original panic message, last log lines, and reiterate the `R` shortcut so the operator isn’t guessing during a live demo.

### QA & Stability
- Teach `counter_e2e.rs` to skip (or install) `cargo-component` when it’s missing, so `cargo test` stays green on clean machines/CI.
- Add a focused unit test around `App::key_event_from_winit` to lock the modifier/character mapping and prevent regressions.
- Run a manual smoke checklist (macOS HiDPI + Linux Wayland) and capture the results in `notes/demo-checklist.md` for future presenters.
