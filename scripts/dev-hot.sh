#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

# shellcheck disable=SC1091
source "$SCRIPT_DIR/lib/test-env.sh"
# shellcheck disable=SC1091
source "$SCRIPT_DIR/lib/dev-ports.sh"
# shellcheck disable=SC1091
source "$SCRIPT_DIR/lib/dev-identity-keys.sh"

load_dev_identity_keys

cd "$ROOT_DIR"

export NVBES_ENVIRONMENT="${NVBES_ENVIRONMENT:-development}"
export NVBES_SMTP_PORT="${NVBES_SMTP_PORT:-1025}"
export NVBES_EMAIL_DATABASE_URL="${NVBES_DEV_EMAIL_DATABASE_URL:-postgres://postgres:postgres@localhost:5432/nvbes_dev_email}"
export NVBES_TRUST_RISK_DATABASE_URL="${NVBES_DEV_TRUST_RISK_DATABASE_URL:-postgres://postgres:postgres@localhost:5432/nvbes_dev_trust_risk}"
export NVBES_IDENTITY_DATABASE_URL="${NVBES_DEV_IDENTITY_DATABASE_URL:-postgres://postgres:postgres@localhost:5432/nvbes_dev_identity}"
export NVBES_ACCOUNT_DATABASE_URL="${NVBES_DEV_ACCOUNT_DATABASE_URL:-postgres://postgres:postgres@localhost:5432/nvbes_dev_account}"
export NVBES_BILLING_DATABASE_URL="${NVBES_DEV_BILLING_DATABASE_URL:-postgres://postgres:postgres@localhost:5432/nvbes_dev_billing}"

# Ports
export NVBES_IDENTITY_BIND_ADDR="${NVBES_IDENTITY_BIND_ADDR:-127.0.0.1:3060}"
export NVBES_ACCOUNT_BIND_ADDR="${NVBES_ACCOUNT_BIND_ADDR:-127.0.0.1:3070}"
export NVBES_TRUST_RISK_BIND_ADDR="${NVBES_TRUST_RISK_BIND_ADDR:-127.0.0.1:3050}"
export NVBES_EMAIL_HTTP_BIND_ADDR="${NVBES_DEV_EMAIL_BIND_ADDR:-127.0.0.1:3040}"
export NVBES_EMAIL_GRPC_BIND_ADDR="$NVBES_EMAIL_HTTP_BIND_ADDR"
export NVBES_BILLING_BIND_ADDR="${NVBES_BILLING_BIND_ADDR:-127.0.0.1:3080}"

node "$SCRIPT_DIR/env.mjs" sync
node "$SCRIPT_DIR/stripe-dev.mjs" --check

# Ensure pnpm dependencies are installed
if [[ ! -d "node_modules" || ! -f "pnpm-lock.yaml" ]]; then
  echo "Installing pnpm dependencies..."
  pnpm install --frozen-lockfile --prefer-offline
fi

# Start infra
docker compose --profile infra -f infrastructure/local/docker-compose.yml up --detach --wait

# Create databases if needed
for database in nvbes_dev nvbes_dev_email nvbes_dev_trust_risk nvbes_dev_identity nvbes_dev_account nvbes_dev_billing; do
  database_exists="$(docker compose --profile infra -f infrastructure/local/docker-compose.yml exec -T postgres \
    psql --username postgres --dbname postgres --tuples-only --no-align \
    --command "SELECT 1 FROM pg_database WHERE datname = '$database'")"
  if [[ "$database_exists" != "1" ]]; then
    docker compose --profile infra -f infrastructure/local/docker-compose.yml exec -T postgres \
      createdb --username postgres "$database"
  fi
done

# Trap for cleanup
cleanup() {
  pkill -P $$ 2>/dev/null || true
  wait 2>/dev/null || true
}

trap cleanup EXIT INT TERM

# Start API and Worker with hotreload in parallel
echo "Starting API and Worker with hotreload..."

bash "$SCRIPT_DIR/dev-api.sh" &
bash "$SCRIPT_DIR/dev-worker.sh" &

# Watch config/env files and trigger env sync
if command -v watchexec >/dev/null 2>&1; then
  watchexec --restart \
    --watch .env \
    --watch .env.example \
    --watch .env.staging.example \
    --watch pnpm-workspace.yaml \
    --watch Cargo.toml \
    --watch Cargo.lock \
    --watch nx.json \
    --watch tsconfig.base.json \
    --watch vite.config.ts \
    --exts env,example,yaml,toml,lock,json,ts \
    -- node "$SCRIPT_DIR/env.mjs" sync &
fi

echo "All services started with hotreload. Press Ctrl+C to stop."
wait
