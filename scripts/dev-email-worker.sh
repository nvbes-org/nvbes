#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
source "$SCRIPT_DIR/lib/test-env.sh"

cd "$ROOT_DIR"

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
