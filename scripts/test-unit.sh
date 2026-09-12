#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib/test-env.sh"

cd "$ROOT_DIR"

if [ "${GITHUB_ACTIONS:-}" != "true" ]; then
	bash "$ROOT_DIR/scripts/ci/setup-test-services.sh"
fi

log_step "web typecheck"
pnpm --dir apps/cloud-web typecheck
pnpm --dir apps/account-web typecheck

log_step "parallel rust unit tests"
cargo test --workspace --exclude nvbes-identity-worker --lib --bins --locked -- \
	--test-threads=4 \
	--skip postgresql_fresh_database_reaches_complete_schema \
	--skip postgresql_upgrade_from_n_minus_one_preserves_compatible_data

log_step "serial identity worker tests"
cargo test --package nvbes-identity-worker --locked -- --test-threads=1
