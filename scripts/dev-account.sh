#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib/test-env.sh"
source "$SCRIPT_DIR/lib/dev-ports.sh"

# Local development captures transactional emails in MailHog instead of using
# the in-memory mock or a real SMTP provider.
export NVBES_EMAIL_PROVIDER="${NVBES_EMAIL_PROVIDER:-smtp}"
export NVBES_EMAIL_FROM_EMAIL="${NVBES_EMAIL_FROM_EMAIL:-dev@nvbes.local}"
# The dev process commonly runs in the devcontainer while MailHog publishes
# its port on the Docker host.
export NVBES_SMTP_HOST="${NVBES_SMTP_HOST:-host.docker.internal}"
export NVBES_SMTP_PORT="${NVBES_SMTP_PORT:-1025}"
export NVBES_SMTP_STARTTLS="${NVBES_SMTP_STARTTLS:-false}"

cd "$ROOT_DIR"

cleanup() {
  terminate_child_jobs
}

trap cleanup INT TERM

if [ -n "${NVBES_OBSERVABILITY_LOG_DIR:-}" ]; then
  mkdir -p "$NVBES_OBSERVABILITY_LOG_DIR"
  bash "$SCRIPT_DIR/dev-account-service.sh" \
    2>&1 | tee -a "$NVBES_OBSERVABILITY_LOG_DIR/account-service.jsonl" &
  bash "$SCRIPT_DIR/dev-account-web.sh" &
  cargo run -p nvbes-account-worker \
    2>&1 | tee -a "$NVBES_OBSERVABILITY_LOG_DIR/account-worker.jsonl" &
else
  bash "$SCRIPT_DIR/dev-account-service.sh" &
  bash "$SCRIPT_DIR/dev-account-web.sh" &
  cargo run -p nvbes-account-worker &
fi

wait
