#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib/test-env.sh"

cd "$ROOT_DIR"

export NVBES_DATABASE_URL="${NVBES_CLOUD_DATABASE_URL:-${NVBES_DATABASE_URL:-postgres://postgres:postgres@localhost:5432/nvbes_cloud}}"
cargo run -p nvbes-cloud-service -- migrate
