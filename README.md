# frontier-wasm

A prototype browser-style app framework: desktop host code loads WebAssembly components with Wasmtime and renders them through Vello.

## Usage

- Enter the Nix environment with `direnv allow` (recommended) or `nix develop`.
- Run the interactive counter demo with `just demo`.
- Run the full CI check suite with `just ci`.

The demo ships with an embedded counter component, so you can launch it without pre-building artifacts. Use `just dist` if you need a shareable bundle.
