#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

COVERAGE_DIR=".temp/rust"
COVERAGE_REPORT="$COVERAGE_DIR/coverage-workspace.json"
THRESHOLDS="docs/testing/rust-coverage-thresholds.json"
IGNORE_REGEX='(\.tests\.rs|\.test_support\.rs)$'
TARGET_DIR="target/llvm-cov-target"

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

# Avoid leaking a developer shell / .env into config.from_env unit tests.
for polluted in NVBES_APP_URL NVBES_WEB_BASE_URL NVBES_API_BASE_URL NVBES_TARGET_ENV; do
  unset "$polluted" || true
done

mkdir -p "$COVERAGE_DIR"
export CARGO_TARGET_DIR="$TARGET_DIR"
rm -rf "$TARGET_DIR"
cargo llvm-cov clean --workspace

# Collect coverage without exporting. On some hosts, `llvm-cov export` SIGSEGVs
# when cargo-llvm-cov also passes build-script objects. Collect first, drop
# those objects, then let cargo-llvm-cov report from the merged profile.
cargo llvm-cov \
  --workspace \
  --all-targets \
  --all-features \
  --locked \
  --ignore-filename-regex "$IGNORE_REGEX" \
  --no-report

rm -rf "$TARGET_DIR/debug/build" "$TARGET_DIR/release/build"

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
