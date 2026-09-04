#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
source "$SCRIPT_DIR/lib/test-env.sh"

cd "$ROOT_DIR"

export NVBES_EMAIL_DATABASE_URL="${NVBES_DEV_EMAIL_DATABASE_URL:-postgres://postgres:postgres@localhost:5432/nvbes_dev_email}"
export NVBES_EMAIL_HTTP_BIND_ADDR="${NVBES_DEV_EMAIL_BIND_ADDR:-127.0.0.1:3040}"
export NVBES_EMAIL_GRPC_BIND_ADDR="$NVBES_EMAIL_HTTP_BIND_ADDR"

pnpm generate:email

if cargo watch --version >/dev/null 2>&1; then
  exec cargo watch \
    --watch apps/email-worker \
    --watch libs/rust/email \
    --watch libs/rust/adapters/email-scaleway \
    --watch libs/ts/email-ui/src \
    --watch Cargo.toml \
    --watch Cargo.lock \
    --ignore libs/rust/email/templates \
    --shell 'pnpm --dir libs/ts/email-ui run generate && cargo run -p nvbes-email-worker'
fi

echo "warning: cargo-watch is unavailable; email worker hot reload is disabled" >&2
exec cargo run -p nvbes-email-worker
