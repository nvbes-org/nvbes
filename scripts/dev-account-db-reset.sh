#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
source "$SCRIPT_DIR/lib/test-env.sh"

cd "$ROOT_DIR"

ACCOUNT_DATABASE_URL="${NVBES_ACCOUNT_DATABASE_URL:-${NVBES_DATABASE_URL:-postgres://postgres:postgres@localhost:5432/nvbes}}"
EXPECTED_LOCAL_URL="postgres://postgres:postgres@localhost:5432/nvbes"

if [ "$ACCOUNT_DATABASE_URL" != "$EXPECTED_LOCAL_URL" ] && [ "${NVBES_ALLOW_ACCOUNT_DB_RESET:-}" != "yes" ]; then
  fail "refusing to reset non-default Account database URL; set NVBES_ALLOW_ACCOUNT_DB_RESET=yes to confirm"
fi

printf 'Resetting local Account database: %s\n' "$ACCOUNT_DATABASE_URL"
sqlx database reset \
  --database-url "$ACCOUNT_DATABASE_URL" \
  --source apps/account-service/migrations \
  --no-dotenv \
  --force \
  -y
