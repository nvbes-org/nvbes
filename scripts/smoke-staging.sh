#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib/test-env.sh"

require_cmd pnpm
require_env NVBES_STAGING_WEB_BASE_URL
require_env NVBES_STAGING_API_BASE_URL
require_env NVBES_STAGING_DATABASE_URL

cd "$ROOT_DIR"

log_step "staging smoke"
NVBES_WEB_BASE_URL="$NVBES_STAGING_WEB_BASE_URL" \
  NVBES_API_BASE_URL="$NVBES_STAGING_API_BASE_URL" \
  pnpm test:smoke

log_step "staging critical E2E"
NVBES_WEB_BASE_URL="$NVBES_STAGING_WEB_BASE_URL" \
  NVBES_API_BASE_URL="$NVBES_STAGING_API_BASE_URL" \
  NVBES_DATABASE_URL="$NVBES_STAGING_DATABASE_URL" \
  pnpm test:e2e:critical
