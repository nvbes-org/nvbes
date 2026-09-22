#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

COVERAGE_DIR=".temp/rust"
COVERAGE_REPORT="$COVERAGE_DIR/coverage-workspace.json"
THRESHOLDS="docs/testing/rust-coverage-thresholds.json"
IGNORE_REGEX='(\.tests\.rs|\.test_support\.rs)$'

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
cargo llvm-cov clean --profraw-only

# Collect coverage without exporting. On some hosts (macOS arm64 and this
# Linux agent), `llvm-cov export` SIGSEGVs when cargo-llvm-cov also passes
# build-script objects from target/llvm-cov-target/debug/build.
cargo llvm-cov \
  --workspace \
  --all-targets \
  --all-features \
  --locked \
  --ignore-filename-regex "$IGNORE_REGEX" \
  --no-report

mapfile -t DEP_OBJECTS < <(
  find target/llvm-cov-target/debug/deps -maxdepth 1 -type f -perm -111 \
    ! -name '*.so' ! -name '*.dylib' ! -name '*.dll' | sort
)
if [[ ${#DEP_OBJECTS[@]} -eq 0 ]]; then
  printf 'error: no instrumented binaries found under target/llvm-cov-target/debug/deps\n' >&2
  exit 1
fi

HOST_TARGET="$(rustc -vV | awk '/^host:/{print $2}')"
LLVM_COV="$(rustup which --toolchain "$(rustup show active-toolchain | awk '{print $1}')" llvm-cov 2>/dev/null || true)"
if [[ -z "$LLVM_COV" ]]; then
  LLVM_COV="$(dirname "$(rustup which rustc)")/../lib/rustlib/${HOST_TARGET}/bin/llvm-cov"
fi
if [[ ! -x "$LLVM_COV" ]]; then
  # Fallback: let rustup resolve the sysroot binary.
  LLVM_COV="$(rustc --print sysroot)/lib/rustlib/${HOST_TARGET}/bin/llvm-cov"
fi
if [[ ! -x "$LLVM_COV" ]]; then
  printf 'error: llvm-cov binary not found for toolchain\n' >&2
  exit 1
fi

PROFDATA="target/llvm-cov-target/workspace.profdata"
if [[ ! -f "$PROFDATA" ]]; then
  # cargo-llvm-cov may leave the merged profile under a dated name; merge as fallback.
  mapfile -t PROFRAW < <(find target/llvm-cov-target -name '*.profraw' | head -200)
  if [[ ${#PROFRAW[@]} -eq 0 ]]; then
    printf 'error: missing coverage profile data\n' >&2
    exit 1
  fi
  LLVM_PROFDATA="$(dirname "$LLVM_COV")/llvm-profdata"
  "$LLVM_PROFDATA" merge -sparse "${PROFRAW[@]}" -o "$PROFDATA"
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
  -summary-only >"$COVERAGE_REPORT"

"$LLVM_COV" export \
  -format=lcov \
  -instr-profile="$PROFDATA" \
  "${OBJECT_ARGS[@]}" \
  -ignore-filename-regex="$IGNORE_REGEX" >"$COVERAGE_DIR/lcov.info"

node tools/rust-workspace/check-coverage.mjs "$COVERAGE_REPORT" "$THRESHOLDS"
