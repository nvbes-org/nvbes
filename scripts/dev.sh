#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
node "$SCRIPT_DIR/env.mjs" sync
# shellcheck disable=SC1091
source "$SCRIPT_DIR/lib/test-env.sh"
# shellcheck disable=SC1091
source "$SCRIPT_DIR/lib/dev-ports.sh"
# shellcheck disable=SC1091
source "$SCRIPT_DIR/lib/dev-identity-keys.sh"

cd "$ROOT_DIR"

load_dev_identity_keys
export NVBES_ENVIRONMENT="${NVBES_ENVIRONMENT:-development}"
export NVBES_SMTP_PORT="${NVBES_SMTP_PORT:-1025}"
export NVBES_EMAIL_DATABASE_URL="${NVBES_DEV_EMAIL_DATABASE_URL:-postgres://postgres:postgres@localhost:5432/nvbes_dev_email}"
export NVBES_TRUST_RISK_DATABASE_URL="${NVBES_DEV_TRUST_RISK_DATABASE_URL:-postgres://postgres:postgres@localhost:5432/nvbes_dev_trust_risk}"
export NVBES_IDENTITY_DATABASE_URL="${NVBES_DEV_IDENTITY_DATABASE_URL:-postgres://postgres:postgres@localhost:5432/nvbes_dev_identity}"
export NVBES_ACCOUNT_DATABASE_URL="${NVBES_DEV_ACCOUNT_DATABASE_URL:-postgres://postgres:postgres@localhost:5432/nvbes_dev_account}"
export NVBES_BILLING_DATABASE_URL="${NVBES_DEV_BILLING_DATABASE_URL:-postgres://postgres:postgres@localhost:5432/nvbes_dev_billing}"
node "$SCRIPT_DIR/stripe-dev.mjs" --check

cleanup() {
  terminate_child_jobs
}

trap cleanup EXIT INT TERM

docker compose --profile infra -f infrastructure/local/docker-compose.yml up --detach --wait

# The init SQL only runs when PostgreSQL creates a fresh volume. Keep startup
# idempotent for developers who already have an existing local volume.
for database in nvbes_dev_email nvbes_dev_trust_risk nvbes_dev_identity nvbes_dev_account nvbes_dev_billing; do
  database_exists="$(docker compose --profile infra -f infrastructure/local/docker-compose.yml exec -T postgres \
    psql --username postgres --dbname postgres --tuples-only --no-align \
    --command "SELECT 1 FROM pg_database WHERE datname = '$database'")"
  if [[ "$database_exists" != "1" ]]; then
    docker compose --profile infra -f infrastructure/local/docker-compose.yml exec -T postgres \
      createdb --username postgres "$database"
  fi
done

cargo run --package nvbes-email-worker -- migrate
cargo run --package nvbes-trust-risk-service -- migrate
cargo run --package nvbes-identity-service -- migrate
cargo run --package nvbes-account-service -- migrate
cargo run --package nvbes-billing-service -- migrate

bash "$SCRIPT_DIR/dev-email-worker.sh" &
bash "$SCRIPT_DIR/dev-trust-risk-service.sh" &
bash "$SCRIPT_DIR/dev-identity-service.sh" &
bash "$SCRIPT_DIR/dev-account-service.sh" &
bash "$SCRIPT_DIR/dev-billing-service.sh" &
wait
