#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
source "$SCRIPT_DIR/lib/test-env.sh"
# shellcheck disable=SC1091
source "$SCRIPT_DIR/lib/dev-ports.sh"

export NVBES_ACCOUNT_DATABASE_URL="${NVBES_ACCOUNT_DATABASE_URL:-postgres://postgres:postgres@localhost:5432/nvbes_account}"
export NVBES_ACCOUNT_SERVICE_PORT="${NVBES_ACCOUNT_SERVICE_PORT:-4001}"
export NVBES_ACCOUNT_SERVICE_BASE_URL="${NVBES_ACCOUNT_SERVICE_BASE_URL:-http://localhost:${NVBES_ACCOUNT_SERVICE_PORT}}"
export NVBES_IDENTITY_SERVICE_BASE_URL="${NVBES_IDENTITY_SERVICE_BASE_URL:-http://localhost:4000}"
export NVBES_ACCOUNT_WEB_BASE_URL="${NVBES_ACCOUNT_WEB_BASE_URL:-http://localhost:3001}"
export NVBES_ACCOUNT_AVATAR_STORAGE_MODE="${NVBES_ACCOUNT_AVATAR_STORAGE_MODE:-mock}"
export NVBES_ENVIRONMENT="${NVBES_ENVIRONMENT:-development}"

cd "$ROOT_DIR"
free_dev_port "$NVBES_ACCOUNT_SERVICE_PORT" "account-service"
exec cargo run -p nvbes-account-service
