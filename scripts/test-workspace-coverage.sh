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

for variable in NVBES_SECURITY_TEST_DATABASE_URL DATABASE_URL; do
  if [[ -z "${!variable:-}" ]]; then
    printf 'error: missing required environment variable: %s\n' "$variable" >&2
    printf 'hint: provision the test PostgreSQL with tools/ci/test-security-database.mjs\n' >&2
    exit 1
  fi
done

mkdir -p "$COVERAGE_DIR"
cargo llvm-cov clean --profraw-only
cargo llvm-cov \
  --workspace \
  --all-targets \
  --all-features \
  --locked \
  --ignore-filename-regex '(\.tests\.rs|\.test_support\.rs)$' \
  --json \
  --summary-only \
  --output-path "$COVERAGE_REPORT"

cargo llvm-cov report \
  --lcov \
  --all-features \
  --ignore-filename-regex '(\.tests\.rs|\.test_support\.rs)$' \
  --output-path "$COVERAGE_DIR/lcov.info"

node tools/rust-workspace/check-coverage.mjs "$COVERAGE_REPORT" "$THRESHOLDS"