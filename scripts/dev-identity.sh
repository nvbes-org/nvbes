#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib/test-env.sh"

cd "$ROOT_DIR"

cleanup() {
  kill 0
}

trap cleanup INT TERM

cargo run -p nvbes-identity-api &
pnpm --dir apps/identity-web dev &
cargo run -p nvbes-identity-worker &
wait
