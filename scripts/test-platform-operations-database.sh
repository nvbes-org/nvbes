#!/usr/bin/env bash
set -euo pipefail
export DATABASE_URL="${DATABASE_URL:-${NVBES_PLATFORM_OPERATIONS_DATABASE_URL:-}}"
: "${DATABASE_URL:?A disposable PostgreSQL test server with CREATEDB is required}"
cargo nextest run --locked --package nvbes-platform --features database-tests
