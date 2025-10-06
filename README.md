# frontier-wasm

This prototype shows a desktop "browser" host that loads WebAssembly components with Wasmtime and renders them through Vello. It ships with a release-built counter guest so the demo works out of the box.

## Quick Start

1. Launch the Nix development shell (`direnv allow` or `nix develop`). It provides Rust, `cargo-component`, Wasmtime, and all build tools.
2. Run the interactive counter window with `just run` (append `release` for a release build). The host will fall back to the embedded counter if no component path is provided.
3. Execute the full check suite with `just ci` before sending changes; it mirrors the GitHub Actions pipeline.

Extra helpers: `just dist` assembles a distributable bundle under `dist/`, and `just ensure-cargo-component` exits early if you forget to run inside the Nix shell.
