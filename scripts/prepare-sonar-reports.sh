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
if command -v cargo-llvm-cov >/dev/null 2>&1; then
  if [[ -z "${NVBES_SECURITY_TEST_DATABASE_URL:-}" || -z "${DATABASE_URL:-}" ]]; then
    printf '==> Wrapping with security test DB for Sonar coverage...\n'
    bash scripts/with-security-test-db.sh bash scripts/test-workspace-coverage.sh
  else
    printf '==> Generating Rust LCOV coverage report...\n'
    bash scripts/test-workspace-coverage.sh
  fi
else
  printf 'error: cargo-llvm-cov required for Sonar coverage\n' >&2
  exit 1
fi

# 3. Generate TypeScript LCOV coverage reports
if command -v pnpm >/dev/null 2>&1; then
  printf '==> Prebuilding TypeScript libraries for consumer apps...\n'
  pnpm --filter "@nvbes/http-client" --filter "@nvbes/identity-sdk-web" build || true

  printf '==> Generating TypeScript LCOV coverage reports...\n'
  pnpm --filter "@nvbes/http-client" \
       --filter "@nvbes/identity-sdk-web" \
       --filter "@nvbes/email-ui" \
       --filter "@nvbes/account-web" \
       --filter "@nvbes/identity-web" \
       exec vitest run --coverage --coverage.reporter=lcov || {
    printf 'warn: typescript coverage collection exited with non-zero status\n' >&2
  }

  printf '==> Normalizing and combining TypeScript LCOV reports...\n'
  node -e "
    const fs = require('node:fs');
    const path = require('node:path');
    const packages = [
      'libs/ts/http-client',
      'libs/ts/identity-sdk-web',
      'libs/ts/email-ui',
      'apps/account-web',
      'apps/identity-web',
    ];
    const rootDir = process.cwd();
    const combined = [];
    for (const pkg of packages) {
      const lcovPath = path.join(rootDir, pkg, 'coverage', 'lcov.info');
      if (fs.existsSync(lcovPath)) {
        const content = fs.readFileSync(lcovPath, 'utf8');
        const normalized = content.replace(/^SF:(.+)$/gm, (match, filePath) => {
          if (path.isAbsolute(filePath)) {
            return 'SF:' + path.relative(rootDir, filePath);
          }
          return 'SF:' + path.join(pkg, filePath);
        });
        fs.writeFileSync(lcovPath, normalized, 'utf8');
        combined.push(normalized);
      }
    }
    if (combined.length > 0) {
      fs.mkdirSync(path.join(rootDir, 'coverage'), { recursive: true });
      fs.writeFileSync(path.join(rootDir, 'coverage', 'lcov.info'), combined.join('\n'), 'utf8');
    }
  "
fi

# 4. Validate generated reports
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

if [[ -f "coverage/lcov.info" ]]; then
  ts_lcov_bytes="$(wc -c < "coverage/lcov.info" | tr -d ' ')"
  printf '  [ok] TypeScript LCOV coverage: coverage/lcov.info (%s bytes)\n' "$ts_lcov_bytes"
fi

printf '==> SonarCloud report preparation complete\n'
