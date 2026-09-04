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
export NVBES_IDENTITY_DATABASE_URL="${NVBES_DEV_IDENTITY_DATABASE_URL:-postgres://postgres:postgres@localhost:5432/nvbes_dev_identity}"
export NVBES_IDENTITY_BIND_ADDR="${NVBES_IDENTITY_BIND_ADDR:-127.0.0.1:3060}"

cd "$ROOT_DIR"
free_dev_port "${NVBES_IDENTITY_BIND_ADDR##*:}" "identity-service"
exec cargo run --package nvbes-identity-service
