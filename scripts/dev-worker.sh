#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
source "$SCRIPT_DIR/lib/test-env.sh"
# shellcheck disable=SC1091
source "$SCRIPT_DIR/lib/dev-ports.sh"

export NVBES_ENVIRONMENT="${NVBES_ENVIRONMENT:-development}"
export DATABASE_URL="${DATABASE_URL:-postgres://postgres:postgres@localhost:5432/nvbes_dev}"
export PORT="${PORT:-8081}"

cd "$ROOT_DIR"
free_dev_port "$PORT" "worker"

if cargo watch --version >/dev/null 2>&1; then
  exec cargo watch \
    --watch apps/worker \
    --watch libs/rust \
    --watch Cargo.toml \
    --watch Cargo.lock \
    --shell 'cargo run -p nvbes-worker'
fi

exec cargo run -p nvbes-worker
