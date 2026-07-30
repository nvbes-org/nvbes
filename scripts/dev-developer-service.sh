#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
source "$SCRIPT_DIR/lib/test-env.sh"
# shellcheck disable=SC1091
source "$SCRIPT_DIR/lib/dev-ports.sh"

export NVBES_DATABASE_URL="${NVBES_DEVELOPER_DATABASE_URL:-${NVBES_ACCOUNT_DATABASE_URL:-postgres://postgres:postgres@localhost:5432/nvbes_account}}"
export NVBES_DEVELOPER_HTTP_PORT="${NVBES_DEVELOPER_HTTP_PORT:-4040}"
export NVBES_DEVELOPER_GRPC_PORT="${NVBES_DEVELOPER_GRPC_PORT:-4041}"
export NVBES_IDENTITY_SERVICE_BASE_URL="${NVBES_IDENTITY_SERVICE_BASE_URL:-http://127.0.0.1:4000}"
export NVBES_DEVELOPER_IDENTITY_CLIENT_ID="${NVBES_DEVELOPER_IDENTITY_CLIENT_ID:-developer-service}"
export NVBES_DEVELOPER_IDENTITY_CLIENT_SECRET="${NVBES_DEVELOPER_IDENTITY_CLIENT_SECRET:-developer-service-secret-key-12345}"

cd "$ROOT_DIR"
free_dev_port "$NVBES_DEVELOPER_HTTP_PORT" "developer-service HTTP"
free_dev_port "$NVBES_DEVELOPER_GRPC_PORT" "developer-service gRPC"
exec cargo run -p nvbes-developer-service
