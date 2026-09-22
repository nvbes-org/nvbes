#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

# Serialize across concurrent agent pre-push / local runs on the same workspace.
mkdir -p "$ROOT_DIR/target"
exec 9>"$ROOT_DIR/target/condition.lock"
if ! flock -w 3600 9; then
  printf 'error: timed out waiting for %s\n' "condition.lock" >&2
  exit 1
fi

CONDITION_DIR=".temp/rust"
CONDITION_REPORT="$CONDITION_DIR/coverage-condition-workspace.json"
THRESHOLDS="docs/testing/rust-condition-thresholds.json"
TOOLCHAIN="nightly-2026-09-09"
IGNORE_REGEX='(\.tests\.rs|\.test_support\.rs)$'
# Isolate from stable coverage (cargo-llvm-cov nests llvm-cov-target under this).
export CARGO_TARGET_DIR="${ROOT_DIR}/target/condition"

if [[ "${NVBES_CONDITION_TOOLCHAIN:-$TOOLCHAIN}" != "$TOOLCHAIN" ]]; then
  printf 'error: branch evidence requires pinned toolchain %s\n' "$TOOLCHAIN" >&2
  exit 1
fi

if ! command -v cargo >/dev/null 2>&1; then
  printf 'error: missing required command: cargo\n' >&2
  exit 1
fi
if ! cargo llvm-cov --version >/dev/null 2>&1; then
  printf 'error: missing required command: cargo-llvm-cov (cargo install cargo-llvm-cov --locked)\n' >&2
  exit 1
fi
if ! rustup run "$TOOLCHAIN" -- rustc --version >/dev/null 2>&1; then
  printf 'error: missing required toolchain: %s (condition coverage requires a nightly toolchain for --branch)\n' "$TOOLCHAIN" >&2
  printf '  install: rustup toolchain install %s --profile minimal\n' "$TOOLCHAIN" >&2
  exit 1
fi

for polluted in NVBES_APP_URL NVBES_WEB_BASE_URL NVBES_API_BASE_URL NVBES_TARGET_ENV; do
  unset "$polluted" || true
done

mkdir -p "$CONDITION_DIR" "$CARGO_TARGET_DIR"
rustup run "$TOOLCHAIN" -- cargo llvm-cov clean --workspace || true

# One-shot report: nightly --branch places instrumented test binaries under
# debug/build/<crate>/out; do not delete that tree before export.
rustup run "$TOOLCHAIN" -- cargo llvm-cov \
  --workspace \
  --all-targets \
  --locked \
  --branch \
  --ignore-filename-regex "$IGNORE_REGEX" \
  --json \
  --summary-only \
  --output-path "$CONDITION_REPORT"

node tools/rust-workspace/check-condition.mjs "$CONDITION_REPORT" "$THRESHOLDS"
