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

log_step "complete Account portfolio gate"
bash scripts/test-account-portfolio.sh

log_step "build gate"
pnpm build

if [ "$target" = "staging" ]; then
  require_env NVBES_STAGING_WEB_BASE_URL
  require_env NVBES_STAGING_API_BASE_URL
  require_env NVBES_STAGING_ACCOUNT_EMAIL
  require_env NVBES_STAGING_ACCOUNT_PASSWORD
  require_env NVBES_STAGING_BILLING_DATABASE_URL
  validate_staging_account_targets

  log_step "staging stripe mapping preflight"
  NVBES_BILLING_DATABASE_URL="$NVBES_STAGING_BILLING_DATABASE_URL" \
    NVBES_ENV=staging \
    pnpm check:stripe-mappings

  log_step "staging smoke gate"
  NVBES_WEB_BASE_URL="$NVBES_STAGING_WEB_BASE_URL" \
    NVBES_API_BASE_URL="$NVBES_STAGING_API_BASE_URL" \
    pnpm test:smoke

  log_step "staging authenticated Account acceptance gate"
  NVBES_WEB_BASE_URL="$NVBES_STAGING_WEB_BASE_URL" \
    pnpm --dir apps/account-web test:e2e:staging-authenticated
else
  if [ "${RELEASE_APPROVED:-}" != "production" ]; then
    fail "production gate requires RELEASE_APPROVED=production"
  fi

  log_step "independent security assurance gate"
  NVBES_PRODUCTION_RELEASE=1 pnpm check:production-security-operations

  log_step "FAPI high-assurance conformance gate"
  pnpm check:fapi-conformance

  log_step "Account portfolio readiness gate"
  pnpm check:account-release-readiness

  require_env ACCOUNT_ACCEPTANCE_EVIDENCE_FILE
  require_env ACCOUNT_ACCEPTANCE_EVIDENCE_SIGNATURE_FILE
  require_env ACCOUNT_ACCEPTANCE_TRUSTED_KEY_ID
  require_env ACCOUNT_ACCEPTANCE_TRUSTED_PUBLIC_KEY_FILE
  require_env ACCOUNT_ACCEPTANCE_TRUSTED_PUBLIC_KEY_SHA256
  require_env ACCOUNT_DEPLOYMENT_EXPECTED_COMPONENT_UIDS
  require_env ACCOUNT_DEPLOYMENT_TRUSTED_KEY_ID
  require_env ACCOUNT_DEPLOYMENT_TRUSTED_PUBLIC_KEY_FILE
  require_env ACCOUNT_DEPLOYMENT_TRUSTED_PUBLIC_KEY_SHA256
  require_env NVBES_RELEASE_SHA
  require_env NVBES_STAGING_WEB_BASE_URL
  require_env NVBES_STAGING_API_BASE_URL
  require_env NVBES_STAGING_ACCOUNT_EMAIL
  require_env NVBES_STAGING_ACCOUNT_PASSWORD
  require_env NVBES_STAGING_ALLOWED_WEB_ORIGINS
  require_env NVBES_STAGING_ALLOWED_API_ORIGINS
  require_env NVBES_PRODUCTION_WEB_BASE_URL
  require_env NVBES_PRODUCTION_API_BASE_URL
  require_env NVBES_PRODUCTION_ALLOWED_ORIGINS
  validate_staging_account_targets

  log_step "signed Account acceptance and deployment evidence gate"
  pnpm check:account-acceptance-evidence

  log_step "production target and deployed release gate"
  pnpm check:account-production-target

  log_step "pre-production staging smoke gate"
  NVBES_WEB_BASE_URL="$NVBES_STAGING_WEB_BASE_URL" \
    NVBES_API_BASE_URL="$NVBES_STAGING_API_BASE_URL" \
    pnpm test:smoke

  log_step "pre-production authenticated Account acceptance gate"
  NVBES_WEB_BASE_URL="$NVBES_STAGING_WEB_BASE_URL" \
    pnpm --dir apps/account-web test:e2e:staging-authenticated

  log_step "post-deploy production smoke gate"
  NVBES_SMOKE_FORBID_REDIRECTS=1 \
  NVBES_WEB_BASE_URL="$NVBES_PRODUCTION_WEB_BASE_URL" \
    NVBES_API_BASE_URL="$NVBES_PRODUCTION_API_BASE_URL" \
    pnpm test:smoke
fi
