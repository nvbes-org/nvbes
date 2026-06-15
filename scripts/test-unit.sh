#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib/test-env.sh"

cd "$ROOT_DIR"

log_step "web typecheck"
pnpm --dir apps/drive-web typecheck
pnpm --dir apps/identity-web typecheck
pnpm --dir libs/ts/identity-sdk-web test

log_step "rust unit tests"
RUST_TEST_THREADS=1 cargo test --workspace --lib --bins --locked -- --test-threads=1
