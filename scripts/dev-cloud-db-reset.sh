#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
source "$SCRIPT_DIR/lib/test-env.sh"

cd "$ROOT_DIR"

CLOUD_DATABASE_URL="${NVBES_CLOUD_DATABASE_URL:-postgres://postgres:postgres@localhost:5432/nvbes_cloud}"
EXPECTED_LOCAL_URL="postgres://postgres:postgres@localhost:5432/nvbes_cloud"

if [ "$CLOUD_DATABASE_URL" != "$EXPECTED_LOCAL_URL" ] && [ "${NVBES_ALLOW_CLOUD_DB_RESET:-}" != "yes" ]; then
  fail "refusing to reset non-default Cloud database URL; set NVBES_ALLOW_CLOUD_DB_RESET=yes to confirm"
fi

printf 'Resetting local Cloud database: %s\n' "$CLOUD_DATABASE_URL"
sqlx database reset \
  --database-url "$CLOUD_DATABASE_URL" \
  --source apps/cloud-service/migrations \
  --no-dotenv \
  --force \
  -y
