#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
node "$SCRIPT_DIR/env.mjs" sync
# shellcheck disable=SC1091
source "$SCRIPT_DIR/lib/test-env.sh"
# shellcheck disable=SC1091
source "$SCRIPT_DIR/lib/dev-ports.sh"

export NVBES_ENVIRONMENT="${NVBES_ENVIRONMENT:-development}"
export NVBES_BILLING_DATABASE_URL="${NVBES_DEV_BILLING_DATABASE_URL:-postgres://postgres:postgres@localhost:5432/nvbes_dev_billing}"
export NVBES_BILLING_HTTP_BIND_ADDR="${NVBES_BILLING_WORKER_BIND_ADDR:-127.0.0.1:3085}"

cd "$ROOT_DIR"
free_dev_port "${NVBES_BILLING_HTTP_BIND_ADDR##*:}" "billing-worker"

if cargo watch --version >/dev/null 2>&1; then
  exec cargo watch \
    --watch apps/billing-worker \
    --watch libs/rust/billing \
    --watch Cargo.toml \
    --watch Cargo.lock \
    --shell 'cargo run -p nvbes-billing-worker'
fi

exec cargo run -p nvbes-billing-worker
