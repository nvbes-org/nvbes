#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

CONDITION_DIR=".temp/rust"
CONDITION_REPORT="$CONDITION_DIR/coverage-condition-workspace.json"
THRESHOLDS="docs/testing/rust-condition-thresholds.json"
TOOLCHAIN="nightly-2026-09-09"
IGNORE_REGEX='(\.tests\.rs|\.test_support\.rs)$'
TARGET_DIR="target/llvm-cov-condition"

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

# Avoid leaking a developer shell / .env into config.from_env unit tests.
for polluted in NVBES_APP_URL NVBES_WEB_BASE_URL NVBES_API_BASE_URL NVBES_TARGET_ENV; do
  unset "$polluted" || true
done

mkdir -p "$CONDITION_DIR"
rustup run "$TOOLCHAIN" -- cargo llvm-cov clean --workspace --target-dir "$TARGET_DIR"
rustup run "$TOOLCHAIN" -- cargo llvm-cov \
  --workspace \
  --all-targets \
  --locked \
  --branch \
  --target-dir "$TARGET_DIR" \
  --ignore-filename-regex "$IGNORE_REGEX" \
  --no-report

mapfile -t DEP_OBJECTS < <(
  find "$TARGET_DIR/debug/deps" -maxdepth 1 -type f -perm -111 \
    ! -name '*.so' ! -name '*.dylib' ! -name '*.dll' | sort
)
if [[ ${#DEP_OBJECTS[@]} -eq 0 ]]; then
  printf 'error: no instrumented binaries found under %s/debug/deps\n' "$TARGET_DIR" >&2
  exit 1
fi

LLVM_COV="$(rustup run "$TOOLCHAIN" -- rustc --print sysroot)/lib/rustlib/$(rustup run "$TOOLCHAIN" -- rustc -vV | awk '/^host:/{print $2}')/bin/llvm-cov"
if [[ ! -x "$LLVM_COV" ]]; then
  printf 'error: llvm-cov binary not found for toolchain %s\n' "$TOOLCHAIN" >&2
  exit 1
fi

PROFDATA="$TARGET_DIR/workspace.profdata"
if [[ ! -f "$PROFDATA" ]]; then
  mapfile -t PROFRAW < <(find "$TARGET_DIR" -name '*.profraw' | head -400)
  if [[ ${#PROFRAW[@]} -eq 0 ]]; then
    printf 'error: missing condition coverage profile data\n' >&2
    exit 1
  fi
  "$(dirname "$LLVM_COV")/llvm-profdata" merge -sparse "${PROFRAW[@]}" -o "$PROFDATA"
fi

OBJECT_ARGS=()
for object in "${DEP_OBJECTS[@]}"; do
  OBJECT_ARGS+=(-object "$object")
done

"$LLVM_COV" export \
  -format=text \
  -instr-profile="$PROFDATA" \
  "${OBJECT_ARGS[@]}" \
  -ignore-filename-regex="$IGNORE_REGEX" \
  -summary-only >"$CONDITION_REPORT"

node tools/rust-workspace/check-condition.mjs "$CONDITION_REPORT" "$THRESHOLDS"
