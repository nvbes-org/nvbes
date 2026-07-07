#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
source "$SCRIPT_DIR/lib/test-env.sh"
# shellcheck disable=SC1091
source "$SCRIPT_DIR/lib/dev-ports.sh"

export NVBES_DATABASE_URL="${NVBES_ENTERPRISE_DATABASE_URL:-${NVBES_ACCOUNT_DATABASE_URL:-postgres://postgres:postgres@localhost:5432/nvbes_account}}"
export NVBES_ENTERPRISE_GRPC_PORT="${NVBES_ENTERPRISE_GRPC_PORT:-4031}"

cd "$ROOT_DIR"
free_dev_port "$NVBES_ENTERPRISE_GRPC_PORT" "enterprise-service gRPC"
exec cargo run -p nvbes-enterprise-service
