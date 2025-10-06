component profile="debug":
    if [ {{profile}} = "release" ]; then \
        cargo component build -p counter-component --release; \
    else \
        cargo component build -p counter-component; \
    fi

run profile="debug" HOST_ARGS="":
    just component {{profile}}
    cargo run -p frontier-wasm-host -- --component target/wasm32-wasip1/{{profile}}/counter_component.wasm {{HOST_ARGS}}

check:
    cargo fmt --all
    cargo clippy --all-targets --all-features
    cargo check
