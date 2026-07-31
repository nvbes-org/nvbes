#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

# shellcheck disable=SC1091
source "$ROOT_DIR/scripts/lib/test-env.sh"

require_cmd cargo
require_cmd node
require_destructive_account_test_database
require_account_test_redis

export RUST_TEST_THREADS=1
exec cargo test --package nvbes-identity-service --locked -- --test-threads=1
