#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib/test-env.sh"

require_cmd tofu

cd "$ROOT_DIR"

log_step "rust integration test targets"
RUST_TEST_THREADS=1 cargo test --workspace --tests --locked
RUST_TEST_THREADS=1 cargo test --manifest-path apps/billing-service/Cargo.toml --tests --locked

log_step "infrastructure validation"
tofu -chdir=infrastructure/environments/development init -backend=false >/dev/null
tofu -chdir=infrastructure/environments/development validate
tofu -chdir=infrastructure/environments/staging init -backend=false >/dev/null
tofu -chdir=infrastructure/environments/staging validate
