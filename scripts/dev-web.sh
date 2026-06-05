#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib/test-env.sh"

cd "$ROOT_DIR"

cleanup() {
  kill 0
}

trap cleanup INT TERM

pnpm --dir apps/drive-web dev &
pnpm --dir apps/identity-web dev &
wait
