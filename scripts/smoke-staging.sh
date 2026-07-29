#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib/test-env.sh"

require_cmd pnpm
require_env NVBES_STAGING_WEB_BASE_URL
require_env NVBES_STAGING_API_BASE_URL
require_env NVBES_STAGING_ACCOUNT_EMAIL
require_env NVBES_STAGING_ACCOUNT_PASSWORD
validate_staging_account_targets

cd "$ROOT_DIR"

log_step "staging smoke"
NVBES_WEB_BASE_URL="$NVBES_STAGING_WEB_BASE_URL" \
  NVBES_API_BASE_URL="$NVBES_STAGING_API_BASE_URL" \
  pnpm test:smoke

log_step "staging authenticated Account acceptance"
NVBES_WEB_BASE_URL="$NVBES_STAGING_WEB_BASE_URL" \
  pnpm --dir apps/account-web test:e2e:staging-authenticated
