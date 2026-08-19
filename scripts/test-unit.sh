#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib/test-env.sh"

cd "$ROOT_DIR"

log_step "web typecheck"
pnpm --dir apps/cloud-web typecheck
pnpm --dir apps/account-web typecheck
pnpm --dir libs/ts/identity-sdk-web test

log_step "rust unit tests"
cargo test --workspace --lib --bins --locked
