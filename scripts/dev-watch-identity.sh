#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
source "$SCRIPT_DIR/lib/test-env.sh"
# shellcheck disable=SC1091
source "$SCRIPT_DIR/lib/dev-ports.sh"

cd "$ROOT_DIR"

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

free_dev_port "${NVBES_API_PORT:-4000}" "identity-api"

bash "$SCRIPT_DIR/dev-identity-web.sh" &
cargo watch \
  -w apps/identity-api/src \
  -w apps/identity-api/migrations \
  -w libs/rust \
  -w Cargo.toml \
  -w Cargo.lock \
  -x "run -p nvbes-identity-api" &
cargo watch \
  -w apps/identity-worker/src \
  -w libs/rust \
  -w Cargo.toml \
  -w Cargo.lock \
  -x "run -p nvbes-identity-worker" &

wait
