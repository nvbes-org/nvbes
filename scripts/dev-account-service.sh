#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
source "$SCRIPT_DIR/lib/test-env.sh"
# shellcheck disable=SC1091
source "$SCRIPT_DIR/lib/dev-ports.sh"
# shellcheck disable=SC1091
source "$SCRIPT_DIR/lib/dev-identity-keys.sh"

load_dev_identity_keys
export NVBES_ENVIRONMENT="${NVBES_ENVIRONMENT:-development}"
export NVBES_ACCOUNT_DATABASE_URL="${NVBES_DEV_ACCOUNT_DATABASE_URL:-postgres://postgres:postgres@localhost:5432/nvbes_dev_account}"
export NVBES_ACCOUNT_BIND_ADDR="${NVBES_ACCOUNT_BIND_ADDR:-127.0.0.1:3070}"
export NVBES_ACCOUNT_PUBLIC_ORIGIN="${NVBES_ACCOUNT_PUBLIC_ORIGIN:-http://127.0.0.1:${NVBES_ACCOUNT_BIND_ADDR##*:}}"

cd "$ROOT_DIR"
free_dev_port "${NVBES_ACCOUNT_BIND_ADDR##*:}" "account-service"
exec cargo run --package nvbes-account-service
