#!/usr/bin/env bash
set -euo pipefail

if ! command -v cargo-component >/dev/null 2>&1; then
  cat >&2 <<'MSG'
error: cargo-component is not installed or not on PATH.
hint: cargo install cargo-component --locked
MSG
  exit 1
fi

if ! cargo component --version >/dev/null 2>&1; then
  cat >&2 <<'MSG'
error: cargo component subcommand is unavailable.
hint: cargo install cargo-component --locked
MSG
  exit 1
fi
