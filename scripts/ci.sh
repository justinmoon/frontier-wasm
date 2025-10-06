#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="${FRONTIER_WASM_CI_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || pwd -P)}"

if [[ ! -f "$ROOT_DIR/Cargo.toml" ]]; then
  printf 'Unable to locate project root (missing Cargo.toml in %s)\n' "$ROOT_DIR" >&2
  exit 1
fi

cd "$ROOT_DIR"

run_step() {
  local description="$1"
  shift
  printf '\n=== %s ===\n' "$description"
  "$@"
}

run_step "Ensuring cargo-component is installed" ./scripts/ensure-cargo-component.sh
run_step "Normalising generated bindings" rustfmt crates/counter-component/src/bindings.rs
run_step "Checking formatting" cargo fmt --all -- --check
run_step "Checking build" cargo check --workspace --all-targets
run_step "Running clippy" cargo clippy --workspace --all-targets -- -D warnings
run_step "Running tests" cargo test
run_step "Building counter component (release)" cargo component build -p counter-component --release
run_step "Building host (release)" cargo build -p frontier-wasm-host --release
run_step "Packaging demo bundle" just dist

printf '\nCI pipeline completed successfully.\n'
