#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
source "$SCRIPT_DIR/lib/test-env.sh"

export NVBES_DATABASE_URL="${NVBES_DRIVE_DATABASE_URL:-postgres://postgres:postgres@localhost:5432/nvbes_drive}"
export NVBES_API_PORT="4002"
exec cargo run -p nvbes-drive-api
