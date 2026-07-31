#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
source "$SCRIPT_DIR/lib/test-env.sh"

cd "$ROOT_DIR"

IDENTITY_DATABASE_URL="${NVBES_IDENTITY_DATABASE_URL:-postgres://postgres:postgres@localhost:5432/nvbes_identity}"
EXPECTED_LOCAL_URL="postgres://postgres:postgres@localhost:5432/nvbes_identity"

if [ "$IDENTITY_DATABASE_URL" != "$EXPECTED_LOCAL_URL" ] && [ "${NVBES_ALLOW_IDENTITY_DB_RESET:-}" != "yes" ]; then
  fail "refusing to reset non-default Identity database URL; set NVBES_ALLOW_IDENTITY_DB_RESET=yes to confirm"
fi

printf 'Resetting local Identity database: %s\n' "$IDENTITY_DATABASE_URL"
sqlx database reset \
  --database-url "$IDENTITY_DATABASE_URL" \
  --source apps/identity-service/migrations \
  --no-dotenv \
  --force \
  -y
