# frontier-wasm

Prototype host and guest for the Vello canvas rail.

## Prerequisites

- Rust toolchain (2021 edition or newer).
- [`cargo-component`](https://github.com/bytecodealliance/cargo-component):
  ```sh
  cargo install cargo-component
  ```

## Building the counter component

Compile the guest component to a Wasm binary using the component model:

```sh
cargo component build -p counter-component
```

The artifact will be written to
`target/wasm32-wasip1/debug/counter_component.wasm` (use
`--release` to produce the release build under `.../release/`).

## Running the host

Launch the desktop host and point it at the component artifact:

```sh
cargo run -p frontier-wasm-host -- \
  --component target/wasm32-wasip1/debug/counter_component.wasm
```

Alternatively, use the `just run` helper:

```sh
just run                # builds debug component and starts the host
just run release        # builds release component and starts the host
```

The host opens a window rendering the counter component, translating
pointer and keyboard input through the Vello canvas bindings.
