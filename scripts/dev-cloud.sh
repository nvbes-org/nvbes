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

bash "$SCRIPT_DIR/dev-cloud-service.sh" &
bash "$SCRIPT_DIR/dev-cloud-web.sh" &
bash "$SCRIPT_DIR/dev-cloud-worker.sh" &
wait
