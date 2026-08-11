#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

if ! command -v cargo >/dev/null 2>&1; then
  printf 'error: missing required command: cargo\n' >&2
  exit 1
fi
cargo llvm-cov --version >/dev/null

cargo llvm-cov clean --workspace
exec cargo llvm-cov \
  --package nvbes-email \
  --all-targets \
  --locked \
  --ignore-filename-regex '(\.tests\.rs|\.test_support\.rs)$' \
  --fail-under-lines 95 \
  --fail-under-functions 90 \
  --fail-under-regions 90
