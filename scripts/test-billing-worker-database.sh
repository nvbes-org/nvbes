#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

# shellcheck disable=SC1091
source "$ROOT_DIR/scripts/lib/test-env.sh"

export DATABASE_URL="${DATABASE_URL:-${NVBES_BILLING_DATABASE_URL:-}}"
require_env DATABASE_URL
node --test "$ROOT_DIR/scripts/validate-billing-test-database.test.mjs"
node "$ROOT_DIR/scripts/validate-billing-test-database.mjs"

cargo nextest run --locked --package nvbes-billing-worker --features database-tests
