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
# In V1, Cloud domain is archived; endpoint is checked only if set
if [ -n "${NVBES_CLOUD_GRPC_ENDPOINT:-}" ]; then
  require_env NVBES_CLOUD_GRPC_ENDPOINT
fi
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
if [ -n "${NVBES_CLOUD_GRPC_ENDPOINT:-}" ]; then
  ACCOUNT_SERVICE_BASE_URL="$NVBES_API_BASE_URL" \
    node "$ROOT_DIR/tools/account-quality/validate-load-target.mjs" cloud
fi

NVBES_ENV="$NVBES_TARGET_ENV" require_destructive_account_test_database


case "$NVBES_EMAIL_TEST_CAPTURE_DIR" in
  /*) ;;
  *) fail "NVBES_EMAIL_TEST_CAPTURE_DIR must be an absolute isolated directory" ;;
esac

log_step "critical browser journeys"
export CI=true
if [ -d "$ROOT_DIR/apps/identity-web" ]; then
  pnpm --dir apps/identity-web test:e2e:critical
else
  printf 'notice: apps/identity-web is archived in V1; browser E2E journeys are inactive for V1 core foundation.\n'
fi
