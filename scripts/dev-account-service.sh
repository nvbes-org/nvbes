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

cd "$ROOT_DIR"
free_dev_port "${NVBES_API_PORT:-4000}" "account-service"
exec cargo run -p nvbes-account-service
