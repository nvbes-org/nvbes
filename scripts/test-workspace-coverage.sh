#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

COVERAGE_DIR=".temp/rust"
COVERAGE_REPORT="$COVERAGE_DIR/coverage-workspace.json"
THRESHOLDS="docs/testing/rust-coverage-thresholds.json"

if ! command -v cargo >/dev/null 2>&1; then
  printf 'error: missing required command: cargo\n' >&2
  exit 1
fi
cargo llvm-cov --version >/dev/null

mkdir -p "$COVERAGE_DIR"
cargo llvm-cov clean --profraw-only
cargo llvm-cov \
  --workspace \
  --all-targets \
  --locked \
  --ignore-filename-regex '(\.tests\.rs|\.test_support\.rs)$' \
  --json \
  --summary-only \
  --output-path "$COVERAGE_REPORT"

cargo llvm-cov report \
  --lcov \
  --ignore-filename-regex '(\.tests\.rs|\.test_support\.rs)$' \
  --output-path "$COVERAGE_DIR/lcov.info"

node tools/rust-workspace/check-coverage.mjs "$COVERAGE_REPORT" "$THRESHOLDS"