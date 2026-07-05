#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib/test-env.sh"

require_cmd cargo
require_env NVBES_STAGING_DATABASE_URL

cd "$ROOT_DIR"

if [ "${NVBES_ALLOW_STAGING_MIGRATION:-}" != "yes" ]; then
  fail "set NVBES_ALLOW_STAGING_MIGRATION=yes after confirming a fresh staging backup or restore point"
fi

log_step "staging migration preflight"
NVBES_ENV=staging \
  NVBES_DATABASE_URL="$NVBES_STAGING_DATABASE_URL" \
  cargo check --workspace

log_step "apply staging database migrations"
NVBES_ENV=staging \
  NVBES_DATABASE_URL="$NVBES_STAGING_DATABASE_URL" \
  cargo run -p nvbes-cloud-service -- migrate

log_step "staging migration complete"
