#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib/test-env.sh"

require_cmd cargo
require_env NVBES_BILLING_DATABASE_URL

NVBES_ENV="${NVBES_ENV:-staging}" \
NVBES_BILLING_DATABASE_URL="$NVBES_BILLING_DATABASE_URL" \
cargo run -q -p nvbes-billing-service --locked -- check-stripe-mappings
