#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
source "$SCRIPT_DIR/lib/test-env.sh"

export NVBES_DATABASE_URL="${NVBES_IDENTITY_DATABASE_URL:-postgres://postgres:postgres@localhost:5432/nvbes_identity}"
export NVBES_IDENTITY_WORKER_INLINE_HOUSEKEEPING_ENABLED="${NVBES_IDENTITY_WORKER_INLINE_HOUSEKEEPING_ENABLED:-false}"

cd "$ROOT_DIR"
exec cargo run -p nvbes-identity-worker
