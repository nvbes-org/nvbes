#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

RUST_REPORT_DIR=".temp/rust"
mkdir -p "$RUST_REPORT_DIR"

printf '==> Preparing SonarCloud analysis reports\n'

# 1. Generate Rust Clippy report in JSON format (offline sqlx macros)
printf '==> Running cargo clippy to generate SonarCloud report...\n'
SQLX_OFFLINE=true cargo clippy --workspace --all-targets --locked --message-format=json \
  > "$RUST_REPORT_DIR/clippy-report.json" || {
  printf 'warn: cargo clippy exited with non-zero status; report generated with available issues\n' >&2
}

# 2. Generate Rust LCOV coverage report.
# Never emit an empty placeholder: Sonar treats that as 0% coverage on new code.
if command -v cargo-llvm-cov >/dev/null 2>&1 \
  && [[ -n "${NVBES_SECURITY_TEST_DATABASE_URL:-}" ]] \
  && [[ -n "${DATABASE_URL:-}" ]]; then
  printf '==> Generating Rust LCOV coverage report...\n'
  bash scripts/test-workspace-coverage.sh
else
  printf 'error: cargo-llvm-cov + NVBES_SECURITY_TEST_DATABASE_URL + DATABASE_URL required for Sonar coverage\n' >&2
  exit 1
fi

# 3. Validate generated reports
printf '==> Verifying SonarCloud reports:\n'
if [[ -f "$RUST_REPORT_DIR/clippy-report.json" ]]; then
  printf '  [ok] Rust Clippy report: %s (%s bytes)\n' "$RUST_REPORT_DIR/clippy-report.json" "$(wc -c < "$RUST_REPORT_DIR/clippy-report.json" | tr -d ' ')"
else
  printf '  [missing] Rust Clippy report not found\n' >&2
fi

lcov_bytes="$(wc -c < "$RUST_REPORT_DIR/lcov.info" | tr -d ' ')"
if [[ "$lcov_bytes" -lt 100 ]]; then
  printf 'error: Rust LCOV coverage is empty (%s bytes)\n' "$lcov_bytes" >&2
  exit 1
fi
printf '  [ok] Rust LCOV coverage: %s (%s bytes)\n' "$RUST_REPORT_DIR/lcov.info" "$lcov_bytes"

printf '==> SonarCloud report preparation complete\n'
