#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
source "$SCRIPT_DIR/lib/test-env.sh"
# shellcheck disable=SC1091
source "$SCRIPT_DIR/lib/dev-ports.sh"

export NVBES_DATABASE_URL="${NVBES_BILLING_DATABASE_URL:-postgres://postgres:postgres@localhost:5432/nvbes_billing}"
export NVBES_API_PORT="${NVBES_API_PORT:-4000}"
export NVBES_BILLING_SERVICE_PORT="${NVBES_BILLING_SERVICE_PORT:-4020}"
export NVBES_BILLING_GRPC_PORT="${NVBES_BILLING_GRPC_PORT:-4021}"

cd "$ROOT_DIR"
free_dev_port "$NVBES_BILLING_SERVICE_PORT" "billing-service"
free_dev_port "$NVBES_BILLING_GRPC_PORT" "billing-service-grpc"
exec cargo run -p nvbes-billing-service
