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
if [ ! -d "node_modules" ] || [ ! -f "pnpm-lock.yaml" ]; then
  echo "Installing pnpm dependencies..."
  pnpm install --frozen-lockfile --prefer-offline
fi

# Start infra
docker compose --profile infra -f infrastructure/local/docker-compose.yml up --detach --wait

# Create databases if needed
for database in nvbes_dev_email nvbes_dev_trust_risk nvbes_dev_identity nvbes_dev_account nvbes_dev_billing; do
  database_exists="$(docker compose --profile infra -f infrastructure/local/docker-compose.yml exec -T postgres \
    psql --username postgres --dbname postgres --tuples-only --no-align \
    --command "SELECT 1 FROM pg_database WHERE datname = '$database'")"
  if [ "$database_exists" != "1" ]; then
    docker compose --profile infra -f infrastructure/local/docker-compose.yml exec -T postgres \
      createdb --username postgres "$database"
  fi
done

# Run migrations
cargo run --package nvbes-email-worker -- migrate
cargo run --package nvbes-trust-risk-service -- migrate
cargo run --package nvbes-identity-service -- migrate
cargo run --package nvbes-account-service -- migrate
cargo run --manifest-path apps/billing-service/Cargo.toml -- migrate

# Trap for cleanup
cleanup() {
  pkill -P $$ 2>/dev/null || true
  wait 2>/dev/null || true
}

trap cleanup EXIT INT TERM

# Start all services with hotreload in parallel
echo "Starting services with hotreload..."

# Email worker with cargo watch
if command -v cargo-watch >/dev/null 2>&1; then
  cargo watch \
    --watch apps/email-worker \
    --watch libs/rust/email \
    --watch libs/rust/adapters/email-scaleway \
    --watch libs/ts/email-ui/src \
    --watch Cargo.toml \
    --watch Cargo.lock \
    --ignore libs/rust/email/templates \
    --shell 'pnpm --dir libs/ts/email-ui run generate && cargo run -p nvbes-email-worker' &
else
  echo "warning: cargo-watch not found, email worker running without hotreload" >&2
  cargo run -p nvbes-email-worker &
fi

# Identity service with cargo watch + systemfd
if command -v cargo-watch >/dev/null 2>&1 && command -v systemfd >/dev/null 2>&1; then
  free_dev_port "${NVBES_IDENTITY_BIND_ADDR##*:}" "identity-service"
  systemfd --no-pid -s "http::$NVBES_IDENTITY_BIND_ADDR" -- cargo watch \
    --watch apps/identity-service \
    --watch libs/rust/core \
    --watch libs/rust/identity-sdk-backend \
    --watch libs/rust/products/identity \
    --watch libs/rust/platform \
    --watch libs/rust/ports \
    --watch libs/rust/audit \
    --watch libs/rust/tenancy \
    --watch libs/rust/region \
    --watch Cargo.toml \
    --watch Cargo.lock \
    --shell 'cargo run -p nvbes-identity-service' &
else
  echo "warning: cargo-watch or systemfd not found, identity service running without hotreload" >&2
  free_dev_port "${NVBES_IDENTITY_BIND_ADDR##*:}" "identity-service"
  cargo run -p nvbes-identity-service &
fi

# Account service with cargo watch + systemfd
if command -v cargo-watch >/dev/null 2>&1 && command -v systemfd >/dev/null 2>&1; then
  free_dev_port "${NVBES_ACCOUNT_BIND_ADDR##*:}" "account-service"
  systemfd --no-pid -s "http::$NVBES_ACCOUNT_BIND_ADDR" -- cargo watch \
    --watch apps/account-service \
    --watch libs/rust/core \
    --watch libs/rust/products/account \
    --watch libs/rust/platform \
    --watch libs/rust/ports \
    --watch libs/rust/audit \
    --watch libs/rust/tenancy \
    --watch libs/rust/region \
    --watch Cargo.toml \
    --watch Cargo.lock \
    --shell 'cargo run -p nvbes-account-service' &
else
  echo "warning: cargo-watch or systemfd not found, account service running without hotreload" >&2
  free_dev_port "${NVBES_ACCOUNT_BIND_ADDR##*:}" "account-service"
  cargo run -p nvbes-account-service &
fi

# Trust-risk service with cargo watch + systemfd
if command -v cargo-watch >/dev/null 2>&1 && command -v systemfd >/dev/null 2>&1; then
  free_dev_port "${NVBES_TRUST_RISK_BIND_ADDR##*:}" "trust-risk-service"
  systemfd --no-pid -s "http::$NVBES_TRUST_RISK_BIND_ADDR" -- cargo watch \
    --watch apps/trust-risk-service \
    --watch libs/rust/core \
    --watch libs/rust/trust-risk \
    --watch libs/rust/platform \
    --watch libs/rust/ports \
    --watch libs/rust/audit \
    --watch libs/rust/tenancy \
    --watch libs/rust/region \
    --watch Cargo.toml \
    --watch Cargo.lock \
    --shell 'cargo run -p nvbes-trust-risk-service' &
else
  echo "warning: cargo-watch or systemfd not found, trust-risk service running without hotreload" >&2
  free_dev_port "${NVBES_TRUST_RISK_BIND_ADDR##*:}" "trust-risk-service"
  cargo run -p nvbes-trust-risk-service &
fi

# Billing service - Node.js script with nodemon or watchexec
if command -v watchexec >/dev/null 2>&1; then
  watchexec --restart \
    --watch apps/billing-service/src \
    --watch libs/rust/billing \
    --watch libs/rust/core \
    --exts rs,toml,json \
    -- node "$SCRIPT_DIR/stripe-dev.mjs" &
else
  echo "warning: watchexec not found, billing service running without hotreload" >&2
  node "$SCRIPT_DIR/stripe-dev.mjs" &
fi

# TypeScript libraries watch mode
echo "Starting TypeScript libraries in watch mode..."
pnpm --filter @nvbes/identity-sdk dev &
pnpm --filter @nvbes/identity-sdk-web dev &
pnpm --filter @nvbes/http-client dev &
pnpm --filter @nvbes/web-runtime dev &
pnpm --filter @nvbes/web-ui dev &
pnpm --filter @nvbes/identity-client dev &
pnpm --filter @nvbes/account-client dev &
pnpm --filter @nvbes/billing-client dev &

# Email UI generate watch (for email templates)
if command -v watchexec >/dev/null 2>&1; then
  watchexec --restart \
    --watch libs/ts/email-ui/src \
    --exts ts,tsx \
    -- pnpm --dir libs/ts/email-ui run generate &
fi

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
else
  echo "warning: watchexec not found, config/env changes won't auto-reload" >&2
fi

echo "All services started with hotreload. Press Ctrl+C to stop."
wait
