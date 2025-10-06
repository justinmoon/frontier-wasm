set shell := ["bash", "-cu"]

ensure-cargo-component:
    ./scripts/ensure-cargo-component.sh

ci:
    nix run .#ci

component profile="debug":
    just ensure-cargo-component
    if [ "{{profile}}" = "release" ]; then \
        cargo component build -p counter-component --release; \
    else \
        cargo component build -p counter-component; \
    fi

run profile="debug" HOST_ARGS="":
    just component {{profile}}
    if [ "{{profile}}" = "release" ]; then \
        cargo run -p frontier-wasm-host --release -- --component target/wasm32-wasip1/release/counter_component.wasm {{HOST_ARGS}}; \
    else \
        cargo run -p frontier-wasm-host -- --component target/wasm32-wasip1/debug/counter_component.wasm {{HOST_ARGS}}; \
    fi

demo HOST_ARGS="":
    just ensure-cargo-component
    cargo component build -p counter-component --release
    cargo run -p frontier-wasm-host --release -- --component target/wasm32-wasip1/release/counter_component.wasm {{HOST_ARGS}}

dist:
    set -euo pipefail
    just ensure-cargo-component
    cargo build -p frontier-wasm-host --release
    cargo component build -p counter-component --release
    rm -rf dist
    mkdir -p dist
    cp target/release/frontier-wasm-host dist/
    cp target/wasm32-wasip1/release/counter_component.wasm dist/
    python3 ./scripts/write_dist_bundle.py

check:
    cargo fmt --all
    cargo clippy --all-targets --all-features
    cargo check
