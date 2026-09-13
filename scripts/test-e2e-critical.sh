#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib/test-env.sh"

require_cmd cargo
require_cmd node
require_cmd pnpm
require_env NVBES_WEB_BASE_URL
require_env NVBES_API_BASE_URL
require_env NVBES_TARGET_ENV
require_env NVBES_DATABASE_URL
require_env NVBES_CLOUD_GRPC_ENDPOINT
require_env NVBES_EMAIL_TEST_CAPTURE_DIR
require_env NVBES_BETA_SEED_EMAIL
require_env NVBES_BETA_SEED_PASSWORD
require_env NVBES_ALLOW_DESTRUCTIVE_TEST_DATABASE

case "$NVBES_TARGET_ENV" in
  ci | local | test) ;;
  *) fail "critical E2E requires an isolated local/CI target" ;;
esac

ACCOUNT_SERVICE_BASE_URL="$NVBES_API_BASE_URL" \
  node "$ROOT_DIR/tools/account-quality/validate-load-target.mjs" web
ACCOUNT_SERVICE_BASE_URL="$NVBES_API_BASE_URL" \
  node "$ROOT_DIR/tools/account-quality/validate-load-target.mjs" service
ACCOUNT_SERVICE_BASE_URL="$NVBES_API_BASE_URL" \
  node "$ROOT_DIR/tools/account-quality/validate-load-target.mjs" cloud

NVBES_ENV="$NVBES_TARGET_ENV" require_destructive_account_test_database


case "$NVBES_EMAIL_TEST_CAPTURE_DIR" in
  /*) ;;
  *) fail "NVBES_EMAIL_TEST_CAPTURE_DIR must be an absolute isolated directory" ;;
esac

log_step "critical browser journeys"
export CI=true
pnpm --dir apps/identity-web test:e2e:critical
