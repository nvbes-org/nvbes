#!/usr/bin/env bash

set -euo pipefail

workspace_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
# shellcheck disable=SC1091
source "$workspace_root/scripts/lib/test-env.sh"
cd "$workspace_root"

: "${NVBES_WEB_BASE_URL:?NVBES_WEB_BASE_URL is required}"
: "${ACCOUNT_SERVICE_BASE_URL:?ACCOUNT_SERVICE_BASE_URL is required}"
: "${NVBES_TARGET_ENV:?NVBES_TARGET_ENV is required}"
: "${NVBES_DATABASE_URL:?NVBES_DATABASE_URL is required for hermetic service and E2E tests}"
: "${NVBES_REDIS_URL:?NVBES_REDIS_URL is required for the Account worker delivery chain}"
: "${NVBES_CLOUD_GRPC_ENDPOINT:?NVBES_CLOUD_GRPC_ENDPOINT is required for critical E2E beta seeding}"
: "${NVBES_BETA_SEED_EMAIL:?NVBES_BETA_SEED_EMAIL is required for critical E2E tests}"
: "${NVBES_BETA_SEED_PASSWORD:?NVBES_BETA_SEED_PASSWORD is required for critical E2E tests}"

case "$NVBES_TARGET_ENV" in
  ci | local | test) ;;
  *)
    echo "The full nightly lane must run in an isolated local/CI environment." >&2
    exit 2
    ;;
esac

node tools/account-quality/validate-load-target.mjs web
node tools/account-quality/validate-load-target.mjs service
node tools/account-quality/validate-load-target.mjs cloud

NVBES_ENV="$NVBES_TARGET_ENV" require_destructive_account_test_database

if [ -n "${DATABASE_URL:-}" ] && [ "$DATABASE_URL" != "$NVBES_DATABASE_URL" ]; then
  echo "DATABASE_URL and NVBES_DATABASE_URL must identify the same isolated database." >&2
  exit 2
fi
export DATABASE_URL="$NVBES_DATABASE_URL"

require_account_test_redis

export CI=true

pnpm exec nx run account-quality:test
pnpm exec nx run-many -t test \
  --projects=account-web,http-client,identity-client,identity-sdk,identity-sdk-core,identity-sdk-web,web-runtime,web-ui
pnpm exec nx run rust-workspace:test
pnpm exec nx run account-worker:test-container-contract
pnpm exec nx run http-client:test:coverage
pnpm exec nx run account-web:test:e2e:compatibility

worker_log="$workspace_root/.temp/account-quality/account-worker.log"
mkdir -p "$(dirname "$worker_log")"
email_capture_directory="$(
  mktemp -d "$workspace_root/.temp/account-quality/email-capture.XXXXXX"
)"
export NVBES_EMAIL_TEST_CAPTURE_DIR="$email_capture_directory"
NVBES_ENV=development NVBES_EMAIL_PROVIDER=test-capture \
  cargo run -q --package nvbes-account-worker --locked >"$worker_log" 2>&1 &
worker_pid=$!
cleanup_worker() {
  kill "$worker_pid" 2>/dev/null || true
  wait "$worker_pid" 2>/dev/null || true
  find "$email_capture_directory" -type f -delete 2>/dev/null || true
  rmdir "$email_capture_directory" 2>/dev/null || true
}
trap cleanup_worker EXIT

pnpm exec nx run account-web:test:e2e:critical
pnpm --dir apps/account-web test:e2e:security
pnpm exec nx run account-web:test:e2e:monkey

K6_PROFILE=load K6_SUITE=account-api \
  bash tools/account-quality/run-account-k6.sh
