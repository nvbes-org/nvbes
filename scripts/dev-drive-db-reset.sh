#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
source "$SCRIPT_DIR/lib/test-env.sh"

cd "$ROOT_DIR"

DRIVE_DATABASE_URL="${NVBES_DRIVE_DATABASE_URL:-postgres://postgres:postgres@localhost:5432/nvbes_drive}"
EXPECTED_LOCAL_URL="postgres://postgres:postgres@localhost:5432/nvbes_drive"

if [ "$DRIVE_DATABASE_URL" != "$EXPECTED_LOCAL_URL" ] && [ "${NVBES_ALLOW_DRIVE_DB_RESET:-}" != "yes" ]; then
  fail "refusing to reset non-default Drive database URL; set NVBES_ALLOW_DRIVE_DB_RESET=yes to confirm"
fi

printf 'Resetting local Drive database: %s\n' "$DRIVE_DATABASE_URL"
sqlx database reset \
  --database-url "$DRIVE_DATABASE_URL" \
  --source apps/drive-api/migrations \
  --no-dotenv \
  --force \
  -y
