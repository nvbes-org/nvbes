#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

THRESHOLDS="docs/testing/rust-mutation-thresholds.json"
OUT_ROOT=".temp/rust"
OUT_DIR="$OUT_ROOT/mutants.out"
OUTCOMES="$OUT_DIR/outcomes.json"
TIMEOUT="${NVBES_MUTATION_TIMEOUT:-600}"
JOBS="${NVBES_MUTATION_JOBS:-}"

if ! command -v cargo >/dev/null 2>&1; then
  printf 'error: missing required command: cargo\n' >&2
  exit 1
fi
if ! MUTANTS_VERSION="$(cargo mutants --version 2>/dev/null)"; then
  printf 'error: missing required command: cargo-mutants (cargo install cargo-mutants --locked --version 27.1.0)\n' >&2
  exit 1
fi

EXPECTED_MUTANTS_VERSION="$(node -e '
  const fs = require("node:fs");
  const config = JSON.parse(fs.readFileSync(process.argv[1], "utf8"));
  process.stdout.write(config.baseline.tool);
' "$THRESHOLDS")"
if [[ "$MUTANTS_VERSION" != "$EXPECTED_MUTANTS_VERSION" ]]; then
  printf 'error: cargo-mutants version mismatch: expected %s, got %s\n' \
    "$EXPECTED_MUTANTS_VERSION" "$MUTANTS_VERSION" >&2
  exit 1
fi

CARGO_PACKAGE_ARGS=()
while IFS= read -r pkg; do
  CARGO_PACKAGE_ARGS+=(--package "$pkg")
done < <(node -e '
  const fs = require("node:fs");
  const [, path] = process.argv;
  const config = JSON.parse(fs.readFileSync(path, "utf8"));
  if (config.schemaVersion !== 1) process.exit(2);
  process.stdout.write(`${Object.keys(config.crates).join("\n")}\n`);
' "$THRESHOLDS")

if [[ "${#CARGO_PACKAGE_ARGS[@]}" -eq 0 ]]; then
  printf 'error: no mutation-gated crate configured in %s\n' "$THRESHOLDS" >&2
  exit 1
fi

CARGO_MUTANTS_ARGS=(
  "${CARGO_PACKAGE_ARGS[@]}"
  --locked
  --all-features
  --timeout "$TIMEOUT"
  --no-times
  -o "$OUT_ROOT"
)
if [[ -n "$JOBS" ]]; then
  if [[ ! "$JOBS" =~ ^[1-9][0-9]*$ ]]; then
    printf 'error: NVBES_MUTATION_JOBS must be a positive integer\n' >&2
    exit 1
  fi
  if [[ "$JOBS" -gt 8 ]]; then
    printf 'error: NVBES_MUTATION_JOBS must not exceed 8\n' >&2
    exit 1
  fi
  CARGO_MUTANTS_ARGS+=(--jobs "$JOBS")
else
  CARGO_MUTANTS_ARGS+=(--jobs 2)
fi

mkdir -p "$OUT_ROOT"
rm -rf "$OUT_DIR"

# Never share CARGO_TARGET_DIR with cargo-mutants scratch copies: cargo may
# report Fresh and re-run an unmutated test binary (false MISSED). Let each
# scratch tree use its own target/.
unset CARGO_TARGET_DIR

set +e
cargo mutants "${CARGO_MUTANTS_ARGS[@]}"
MUTANTS_STATUS=$?
set -e

# cargo-mutants uses 2 for missed mutants and 3 for timeouts. Both outcomes
# must reach the nvbes threshold checker; all other non-zero statuses are
# execution failures rather than mutation-score results.
if [[ "$MUTANTS_STATUS" -ne 0 && "$MUTANTS_STATUS" -ne 2 && "$MUTANTS_STATUS" -ne 3 ]]; then
  printf 'error: cargo-mutants failed with exit code %s\n' "$MUTANTS_STATUS" >&2
  exit "$MUTANTS_STATUS"
fi

if [[ ! -f "$OUTCOMES" ]]; then
  printf 'error: cargo-mutants did not produce %s\n' "$OUTCOMES" >&2
  exit 1
fi

node "$ROOT_DIR/tools/rust-workspace/check-mutation.mjs" "$OUTCOMES" "$THRESHOLDS"
