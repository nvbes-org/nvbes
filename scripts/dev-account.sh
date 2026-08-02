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

if [ -n "${NVBES_OBSERVABILITY_LOG_DIR:-}" ]; then
  mkdir -p "$NVBES_OBSERVABILITY_LOG_DIR"
  bash "$SCRIPT_DIR/dev-account-service.sh" \
    2>&1 | tee -a "$NVBES_OBSERVABILITY_LOG_DIR/account-service.jsonl" &
  bash "$SCRIPT_DIR/dev-email-worker.sh" &
  bash "$SCRIPT_DIR/dev-account-web.sh" &
  cargo run -p nvbes-account-worker \
    2>&1 | tee -a "$NVBES_OBSERVABILITY_LOG_DIR/account-worker.jsonl" &
else
  bash "$SCRIPT_DIR/dev-account-service.sh" &
  bash "$SCRIPT_DIR/dev-email-worker.sh" &
  bash "$SCRIPT_DIR/dev-account-web.sh" &
  cargo run -p nvbes-account-worker &
fi

wait
