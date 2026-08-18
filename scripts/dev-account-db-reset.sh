#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
source "$SCRIPT_DIR/lib/test-env.sh"

cd "$ROOT_DIR"

ACCOUNT_DATABASE_URL="${NVBES_ACCOUNT_DATABASE_URL:-postgres://postgres:postgres@localhost:5432/nvbes_account}"
EXPECTED_LOCAL_URL="postgres://postgres:postgres@localhost:5432/nvbes_account"
EXPECTED_DEVCONTAINER_URL="postgres://postgres:postgres@postgres:5432/nvbes_account"

if [ "$ACCOUNT_DATABASE_URL" != "$EXPECTED_LOCAL_URL" ] \
  && [ "$ACCOUNT_DATABASE_URL" != "$EXPECTED_DEVCONTAINER_URL" ] \
  && [ "${NVBES_ALLOW_ACCOUNT_DB_RESET:-}" != "yes" ]; then
  fail "refusing to reset non-default Account database URL; set NVBES_ALLOW_ACCOUNT_DB_RESET=yes to confirm"
fi

printf 'Resetting local Account database: %s\n' "$ACCOUNT_DATABASE_URL"
sqlx database reset \
  --database-url "$ACCOUNT_DATABASE_URL" \
  --source apps/account-service-next/migrations \
  --no-dotenv \
  --force \
  -y
