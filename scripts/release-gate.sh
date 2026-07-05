#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib/test-env.sh"

require_cmd pnpm
require_cmd tofu

target="${1:-}"
case "$target" in
  staging | production) ;;
  *) fail "usage: scripts/release-gate.sh <staging|production>" ;;
esac

cd "$ROOT_DIR"

log_step "format gate"
pnpm format:check

log_step "lint gate"
pnpm lint

log_step "compile and type gate"
pnpm check

log_step "unit gate"
pnpm test:unit

log_step "integration gate"
pnpm test:integration

log_step "build gate"
pnpm build

if [ "$target" = "staging" ]; then
  require_env NVBES_STAGING_WEB_BASE_URL
  require_env NVBES_STAGING_API_BASE_URL
  require_env NVBES_STAGING_DATABASE_URL
  require_env NVBES_STAGING_BILLING_DATABASE_URL

  log_step "staging stripe mapping preflight"
  NVBES_BILLING_DATABASE_URL="$NVBES_STAGING_BILLING_DATABASE_URL" \
    NVBES_ENV=staging \
    pnpm check:stripe-mappings

  log_step "staging smoke gate"
  NVBES_WEB_BASE_URL="$NVBES_STAGING_WEB_BASE_URL" \
    NVBES_API_BASE_URL="$NVBES_STAGING_API_BASE_URL" \
    pnpm test:smoke

  log_step "staging critical E2E gate"
  NVBES_WEB_BASE_URL="$NVBES_STAGING_WEB_BASE_URL" \
    NVBES_API_BASE_URL="$NVBES_STAGING_API_BASE_URL" \
    NVBES_DATABASE_URL="$NVBES_STAGING_DATABASE_URL" \
    pnpm test:e2e:critical
else
  if [ "${RELEASE_APPROVED:-}" != "production" ]; then
    fail "production gate requires RELEASE_APPROVED=production"
  fi
  require_env NVBES_STAGING_WEB_BASE_URL
  require_env NVBES_STAGING_API_BASE_URL
  require_env NVBES_STAGING_DATABASE_URL

  log_step "pre-production staging smoke gate"
  NVBES_WEB_BASE_URL="$NVBES_STAGING_WEB_BASE_URL" \
    NVBES_API_BASE_URL="$NVBES_STAGING_API_BASE_URL" \
    pnpm test:smoke

  log_step "pre-production critical E2E gate"
  NVBES_WEB_BASE_URL="$NVBES_STAGING_WEB_BASE_URL" \
    NVBES_API_BASE_URL="$NVBES_STAGING_API_BASE_URL" \
    NVBES_DATABASE_URL="$NVBES_STAGING_DATABASE_URL" \
    pnpm test:e2e:critical

  if [ -n "${NVBES_PRODUCTION_WEB_BASE_URL:-}" ] && [ -n "${NVBES_PRODUCTION_API_BASE_URL:-}" ]; then
    log_step "post-deploy production smoke gate"
    NVBES_WEB_BASE_URL="$NVBES_PRODUCTION_WEB_BASE_URL" \
      NVBES_API_BASE_URL="$NVBES_PRODUCTION_API_BASE_URL" \
      pnpm test:smoke
  else
    printf '\nProduction URLs are not set. Run post-deploy smoke with NVBES_PRODUCTION_WEB_BASE_URL and NVBES_PRODUCTION_API_BASE_URL.\n'
  fi
fi
