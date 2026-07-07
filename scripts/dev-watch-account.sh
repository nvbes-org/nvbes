#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
source "$SCRIPT_DIR/lib/test-env.sh"
# shellcheck disable=SC1091
source "$SCRIPT_DIR/lib/dev-ports.sh"

cd "$ROOT_DIR"

export NVBES_BILLING_GRPC_ENDPOINT="${NVBES_BILLING_GRPC_ENDPOINT:-http://127.0.0.1:4021}"
export NVBES_CLOUD_GRPC_ENDPOINT="${NVBES_CLOUD_GRPC_ENDPOINT:-http://127.0.0.1:4003}"
export NVBES_DEVELOPER_GRPC_ENDPOINT="${NVBES_DEVELOPER_GRPC_ENDPOINT:-http://127.0.0.1:4041}"
export NVBES_ENTERPRISE_GRPC_ENDPOINT="${NVBES_ENTERPRISE_GRPC_ENDPOINT:-http://127.0.0.1:4031}"

if ! command -v cargo-watch >/dev/null 2>&1; then
  log_step "Installing missing Rust watcher: cargo-watch"
  export CARGO_HOME="${NVBES_USER_CARGO_HOME:-$HOME/.cargo}"
  export PATH="$CARGO_HOME/bin:$PATH"
  cargo install cargo-watch --locked
fi

cleanup() {
  terminate_child_jobs
}

trap cleanup EXIT INT TERM

free_dev_port "${NVBES_API_PORT:-4000}" "account-service"

bash "$SCRIPT_DIR/dev-account-web.sh" &
cargo watch \
  -w apps/account-service/src \
  -w apps/account-service/migrations \
  -w libs/rust \
  -w Cargo.toml \
  -w Cargo.lock \
  -x "run -p nvbes-account-service" &
cargo watch \
  -w apps/account-worker/src \
  -w libs/rust \
  -w Cargo.toml \
  -w Cargo.lock \
  -x "run -p nvbes-account-worker" &

wait
