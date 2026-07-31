#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
source "$SCRIPT_DIR/lib/test-env.sh"
# shellcheck disable=SC1091
source "$SCRIPT_DIR/lib/dev-ports.sh"

export NVBES_BILLING_GRPC_ENDPOINT="${NVBES_BILLING_GRPC_ENDPOINT:-http://127.0.0.1:4021}"
export NVBES_CLOUD_GRPC_ENDPOINT="${NVBES_CLOUD_GRPC_ENDPOINT:-http://127.0.0.1:4003}"
export NVBES_DEVELOPER_GRPC_ENDPOINT="${NVBES_DEVELOPER_GRPC_ENDPOINT:-http://127.0.0.1:4041}"
export NVBES_ENTERPRISE_GRPC_ENDPOINT="${NVBES_ENTERPRISE_GRPC_ENDPOINT:-http://127.0.0.1:4031}"
export NVBES_DATABASE_URL="${NVBES_IDENTITY_DATABASE_URL:-postgres://postgres:postgres@localhost:5432/nvbes_identity}"
export NVBES_API_PORT="${NVBES_IDENTITY_SERVICE_PORT:-4000}"
export NVBES_API_BASE_URL="${NVBES_IDENTITY_SERVICE_BASE_URL:-http://localhost:4000}"
export NVBES_WEB_BASE_URL="${NVBES_IDENTITY_WEB_BASE_URL:-http://localhost:3000}"

cd "$ROOT_DIR"
free_dev_port "$NVBES_API_PORT" "identity-service"
exec cargo run -p nvbes-identity-service
