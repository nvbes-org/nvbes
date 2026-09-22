#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

COVERAGE_DIR=".temp/rust"
COVERAGE_REPORT="$COVERAGE_DIR/coverage-workspace.json"
THRESHOLDS="docs/testing/rust-coverage-thresholds.json"
IGNORE_REGEX='(\.tests\.rs|\.test_support\.rs)$'
# Keep stable coverage away from nightly condition profdata (format mismatch).
export CARGO_TARGET_DIR="${ROOT_DIR}/target/coverage-cov"
TARGET_DIR="${CARGO_TARGET_DIR}/llvm-cov-target"

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

for polluted in NVBES_APP_URL NVBES_WEB_BASE_URL NVBES_API_BASE_URL NVBES_TARGET_ENV; do
  unset "$polluted" || true
done

mkdir -p "$COVERAGE_DIR" "$CARGO_TARGET_DIR"
cargo llvm-cov clean --workspace || true
# Best-effort wipe; concurrent scanners can leave non-empty dirs momentarily.
chmod -R u+w "$TARGET_DIR" 2>/dev/null || true
rm -rf "$TARGET_DIR" 2>/dev/null || true
mkdir -p "$TARGET_DIR"

cargo llvm-cov \
  --workspace \
  --all-targets \
  --all-features \
  --locked \
  --no-report

# Drop build-script artifacts that make llvm-cov export SIGSEGV on some hosts.
# Only remove known compiler/build-script crate outputs under debug/build — never
# wipe the whole tree (nightly branch mode stores test bins there).
if [[ -d "$TARGET_DIR/debug/build" ]]; then
  find "$TARGET_DIR/debug/build" -mindepth 1 -maxdepth 1 -type d ! -name 'nvbes-*' \
    -exec rm -rf {} + 2>/dev/null || true
fi
rm -rf "$TARGET_DIR/release/build" 2>/dev/null || true

cargo llvm-cov report \
  --ignore-filename-regex "$IGNORE_REGEX" \
  --json \
  --summary-only \
  --output-path "$COVERAGE_REPORT"

cargo llvm-cov report \
  --ignore-filename-regex "$IGNORE_REGEX" \
  --lcov \
  --output-path "$COVERAGE_DIR/lcov.info"

node tools/rust-workspace/check-coverage.mjs "$COVERAGE_REPORT" "$THRESHOLDS"
