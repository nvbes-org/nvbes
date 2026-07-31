#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

ADMIN_URL="${IDENTITY_MIGRATION_TEST_ADMIN_URL:-${DATABASE_URL:-${NVBES_IDENTITY_DATABASE_URL:-}}}"
if [[ -z "$ADMIN_URL" ]]; then
  echo "Set IDENTITY_MIGRATION_TEST_ADMIN_URL, DATABASE_URL, or NVBES_IDENTITY_DATABASE_URL." >&2
  exit 1
fi

export IDENTITY_MIGRATION_TEST_ADMIN_URL="$ADMIN_URL"
export IDENTITY_MIGRATION_TEST_RUN_ID
IDENTITY_MIGRATION_TEST_RUN_ID="$(
  LC_ALL=C od -An -N8 -tx1 /dev/urandom | tr -d '[:space:]'
)"

cleanup() {
  cargo test \
    --package nvbes-identity-service \
    --locked \
    database_migrations_tests::cleanup_ephemeral_databases_for_current_run \
    -- \
    --exact \
    --ignored \
    --nocapture
}

finalize_exit() {
  local test_status=$?
  trap - EXIT INT TERM
  if ! cleanup; then
    echo "Identity migration test resource cleanup failed." >&2
    test_status=1
  fi
  exit "$test_status"
}

finalize_signal() {
  local signal_status="$1"
  trap - EXIT INT TERM
  if ! cleanup; then
    echo "Identity migration test resource cleanup failed after a signal." >&2
  fi
  exit "$signal_status"
}

trap finalize_exit EXIT
trap 'finalize_signal 130' INT
trap 'finalize_signal 143' TERM

cargo test \
  --package nvbes-identity-service \
  --locked \
  database_migrations_tests:: \
  -- \
  --test-threads=1 \
  --nocapture

# Leave one database and its persisted role snapshot behind in a completed test
# process. The EXIT cleanup below must recover the same state as after an
# interrupted process, without deleting a role that predated this test run.
cargo test \
  --package nvbes-identity-service \
  --locked \
  database_migrations_tests::prepare_interrupted_cluster_state_for_recovery_test \
  -- \
  --exact \
  --ignored \
  --nocapture
