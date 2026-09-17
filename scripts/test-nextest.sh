#!/usr/bin/env bash

set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

if ! command -v cargo-nextest >/dev/null 2>&1; then
  printf 'error: cargo-nextest is not installed. Install it with: cargo install cargo-nextest --version 0.9.144\n' >&2
  exit 1
fi

exec cargo nextest run --locked "$@"
