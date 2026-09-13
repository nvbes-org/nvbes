#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

# shellcheck disable=SC1091
source "$ROOT_DIR/scripts/lib/test-env.sh"

export DATABASE_URL="${DATABASE_URL:-${NVBES_IDENTITY_DATABASE_URL:-}}"
require_env DATABASE_URL
node --test "$ROOT_DIR/scripts/validate-identity-test-database.test.mjs"
node "$ROOT_DIR/scripts/validate-identity-test-database.mjs"

cargo test --locked --package nvbes-identity-service --features database-tests
