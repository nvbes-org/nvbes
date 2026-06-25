#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib/test-env.sh"
source "$SCRIPT_DIR/lib/dev-ports.sh"

cd "$ROOT_DIR"

cleanup() {
  terminate_child_jobs
}

trap cleanup INT TERM

bash "$SCRIPT_DIR/dev-drive-web.sh" &
bash "$SCRIPT_DIR/dev-identity-web.sh" &
wait
