#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
source "$SCRIPT_DIR/lib/test-env.sh"
# shellcheck disable=SC1091
source "$SCRIPT_DIR/lib/dev-ports.sh"

export NVBES_ENVIRONMENT="${NVBES_ENVIRONMENT:-development}"
export NVBES_TRUST_RISK_DATABASE_URL="${NVBES_DEV_TRUST_RISK_DATABASE_URL:-postgres://postgres:postgres@localhost:5432/nvbes_dev_trust_risk}"
export NVBES_TRUST_RISK_BIND_ADDR="${NVBES_TRUST_RISK_BIND_ADDR:-127.0.0.1:3050}"

cd "$ROOT_DIR"
free_dev_port "${NVBES_TRUST_RISK_BIND_ADDR##*:}" "trust-risk-service"
exec cargo run --package nvbes-trust-risk-service
