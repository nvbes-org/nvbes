#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
source "$SCRIPT_DIR/lib/test-env.sh"
# shellcheck disable=SC1091
source "$SCRIPT_DIR/lib/dev-ports.sh"

export NVBES_DATABASE_URL="${NVBES_CLOUD_DATABASE_URL:-postgres://postgres:postgres@localhost:5432/nvbes_cloud}"
export NVBES_API_PORT="4002"
cd "$ROOT_DIR"
free_dev_port "$NVBES_API_PORT" "cloud-service"
exec cargo run -p nvbes-cloud-service
