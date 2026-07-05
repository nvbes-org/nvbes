#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
source "$SCRIPT_DIR/lib/test-env.sh"
# shellcheck disable=SC1091
source "$SCRIPT_DIR/lib/dev-ports.sh"

export NVBES_DATABASE_URL="${NVBES_BILLING_DATABASE_URL:-postgres://postgres:postgres@localhost:5432/nvbes_billing}"
export NVBES_API_PORT="${NVBES_API_PORT:-4000}"
export NVBES_BILLING_API_PORT="${NVBES_BILLING_API_PORT:-4020}"

cd "$ROOT_DIR"
free_dev_port "$NVBES_BILLING_API_PORT" "billing-api"
exec cargo run -p nvbes-billing-api
