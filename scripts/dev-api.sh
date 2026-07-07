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

bash "$SCRIPT_DIR/dev-account-service.sh" &
bash "$SCRIPT_DIR/dev-billing-service.sh" &
bash "$SCRIPT_DIR/dev-developer-service.sh" &
bash "$SCRIPT_DIR/dev-enterprise-service.sh" &
bash "$SCRIPT_DIR/dev-gateway-cloud.sh" &
bash "$SCRIPT_DIR/dev-billing-worker.sh" &
bash "$SCRIPT_DIR/dev-worker.sh" &
bash "$SCRIPT_DIR/dev-cloud-service.sh" &
wait
