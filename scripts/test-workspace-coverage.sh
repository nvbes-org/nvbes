#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

# Serialize across concurrent agent pre-push / local runs on the same workspace.
mkdir -p "$ROOT_DIR/target"
exec 9>"$ROOT_DIR/target/coverage.lock"
if ! flock -w 3600 9; then
  printf 'error: timed out waiting for %s\n' "coverage.lock" >&2
  exit 1
fi

COVERAGE_DIR=".temp/rust"
COVERAGE_REPORT="$COVERAGE_DIR/coverage-workspace.json"
THRESHOLDS="docs/testing/rust-coverage-thresholds.json"
IGNORE_REGEX='(\.tests\.rs|\.test_support\.rs)$'
# Isolate from nightly condition (cargo-llvm-cov nests llvm-cov-target under this).
export CARGO_TARGET_DIR="${ROOT_DIR}/target/coverage"

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

# One-shot line coverage on stable. Avoid wiping CARGO_TARGET_DIR between clean
# and collect: an empty tree without CACHEDIR.TAG races rustc out-dirs.
cargo llvm-cov \
  --workspace \
  --all-targets \
  --all-features \
  --locked \
  --ignore-filename-regex "$IGNORE_REGEX" \
  --json \
  --summary-only \
  --output-path "$COVERAGE_REPORT"

cargo llvm-cov report \
  --ignore-filename-regex "$IGNORE_REGEX" \
  --lcov \
  --output-path "$COVERAGE_DIR/lcov.info"

node tools/rust-workspace/check-coverage.mjs "$COVERAGE_REPORT" "$THRESHOLDS"
