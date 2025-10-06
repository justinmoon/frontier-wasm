# Demo Smoke Checklist

This log captures quick checks to keep the counter demo healthy on presenter machines.
If an item is unchecked, jot down whether it was skipped (and why) or failed.

## macOS (HiDPI)
- [ ] `just demo` succeeds and opens a window on a Retina display.
- [ ] Embedded component loads without `--component` override.
- [ ] Counter responds to mouse scroll and +/- keys.
- [ ] `R` reloads after forcing a panic (e.g., edit component to `panic!`).
- [ ] Error overlay shows panic reason and recent logs.

**Status:** Not run (headless CI environment).

## Linux (Wayland)
- [ ] `just demo` succeeds inside a Wayland session.
- [ ] Window renders pixels at expected scale (no blurry texture).
- [ ] Pointer input and +/- keys adjust the counter.
- [ ] `R` restart recovers from a synthetic failure.
- [ ] Overlay renders text legibly under Wayland font defaults.

**Status:** Not run (needs manual Wayland workstation).

_Add the date/operator each time you run the checklist._
