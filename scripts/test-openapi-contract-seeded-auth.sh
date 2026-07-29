#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib/test-env.sh"

require_cmd cargo
require_cmd node
require_env NVBES_DATABASE_URL
require_env NVBES_WEB_BASE_URL
require_env NVBES_API_BASE_URL
require_env NVBES_REDIS_URL
require_env NVBES_CLOUD_GRPC_ENDPOINT
require_env NVBES_ALLOW_DESTRUCTIVE_TEST_DATABASE

export NVBES_ENV="${NVBES_ENV:-test}"
export NVBES_TARGET_ENV="$NVBES_ENV"
ACCOUNT_SERVICE_BASE_URL="$NVBES_API_BASE_URL" \
  node "$ROOT_DIR/tools/account-quality/validate-load-target.mjs" web
ACCOUNT_SERVICE_BASE_URL="$NVBES_API_BASE_URL" \
  node "$ROOT_DIR/tools/account-quality/validate-load-target.mjs" service
ACCOUNT_SERVICE_BASE_URL="$NVBES_API_BASE_URL" \
  node "$ROOT_DIR/tools/account-quality/validate-load-target.mjs" cloud
require_destructive_account_test_database
require_account_test_redis
NVBES_SMOKE_CONTRACT_SEEDED_AUTH=1 node "$SCRIPT_DIR/test-openapi-contract.mjs"
