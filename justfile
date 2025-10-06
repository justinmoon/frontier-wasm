component-release:
    cargo component build -p counter-component --release

run HOST_ARGS="":
    just component-release
    cargo run -p frontier-wasm-host -- --component target/wasm32-wasip1/release/counter-component.wasm {{HOST_ARGS}}

wasm-dev:
    cargo component build -p counter-component

check:
    cargo fmt --all
    cargo clippy --all-targets --all-features
    cargo check
