#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

# Serialize across concurrent agent pre-push / local runs on the same workspace.
# shellcheck source=lib/workspace-lock.sh
source "$ROOT_DIR/scripts/lib/workspace-lock.sh"
workspace_lock_acquire "$ROOT_DIR/target/condition.lock" 3600

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

mkdir -p "$CONDITION_DIR" "$CARGO_TARGET_DIR"
rustup run "$TOOLCHAIN" -- cargo llvm-cov clean --workspace || true

# Align with the V1 campaign line measure: --all-features + Postgres so
# database-tests modules and sqlx suites contribute branch evidence.
rustup run "$TOOLCHAIN" -- cargo llvm-cov \
  --workspace \
  --all-targets \
  --all-features \
  --locked \
  --branch \
  --ignore-filename-regex "$IGNORE_REGEX" \
  --json \
  --summary-only \
  --output-path "$CONDITION_REPORT"

node tools/rust-workspace/check-condition.mjs "$CONDITION_REPORT" "$THRESHOLDS"
