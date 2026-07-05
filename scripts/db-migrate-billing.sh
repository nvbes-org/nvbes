#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
source "$SCRIPT_DIR/lib/test-env.sh"

cd "$ROOT_DIR"

export NVBES_BILLING_DATABASE_URL="${NVBES_BILLING_DATABASE_URL:-postgres://postgres:postgres@localhost:5432/nvbes_billing}"
cargo run -p nvbes-billing-api -- migrate
