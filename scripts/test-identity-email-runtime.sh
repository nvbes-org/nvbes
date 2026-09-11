#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
export DATABASE_URL="${DATABASE_URL:-${NVBES_IDENTITY_DATABASE_URL:-}}"
node scripts/validate-identity-test-database.mjs
cargo build --locked --package nvbes-email-worker --bin nvbes-email-worker --target-dir target
export NVBES_IDENTITY_TEST_EMAIL_BINARY="$ROOT_DIR/target/debug/nvbes-email-worker"
cargo test --locked --package nvbes-identity-service --features email-integration-tests --lib notification_runtime_tests -- --test-threads=1
