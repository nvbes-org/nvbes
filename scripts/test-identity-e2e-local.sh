#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
source "$SCRIPT_DIR/lib/test-env.sh"

require_cmd cargo
require_cmd curl
require_cmd docker
require_cmd pgrep
require_cmd pnpm

cd "$ROOT_DIR"

COMPOSE_FILE="$ROOT_DIR/infrastructure/local/identity-e2e.compose.yml"
COMPOSE_PROJECT="${NVBES_IDENTITY_E2E_COMPOSE_PROJECT:-nvbes-identity-e2e}"
POSTGRES_PORT="${NVBES_IDENTITY_E2E_POSTGRES_PORT:-55432}"
REDIS_PORT="${NVBES_IDENTITY_E2E_REDIS_PORT:-56379}"
IDENTITY_HTTP_PORT="${NVBES_IDENTITY_E2E_HTTP_PORT:-44000}"
IDENTITY_GRPC_PORT="${NVBES_IDENTITY_E2E_GRPC_PORT:-44010}"
CLOUD_HTTP_PORT="${NVBES_IDENTITY_E2E_CLOUD_HTTP_PORT:-44002}"
CLOUD_GRPC_PORT="${NVBES_IDENTITY_E2E_CLOUD_GRPC_PORT:-44003}"
IDENTITY_WEB_PORT="${NVBES_IDENTITY_E2E_WEB_PORT:-43000}"
WORKER_METRICS_PORT="${NVBES_IDENTITY_E2E_WORKER_METRICS_PORT:-44102}"
RUNTIME_DIR="$(mktemp -d "${TMPDIR:-/tmp}/nvbes-identity-e2e.XXXXXX")"
EMAIL_CAPTURE_DIR="$RUNTIME_DIR/email-captures"
ARTIFACT_DIR="${NVBES_IDENTITY_E2E_ARTIFACT_DIR:-}"
PROCESS_IDS=()
LAST_PROCESS_ID=""

mkdir -p "$EMAIL_CAPTURE_DIR"

compose() {
  docker compose --project-name "$COMPOSE_PROJECT" --file "$COMPOSE_FILE" "$@"
}

terminate_process_tree() {
  local process_id="$1"
  local child_id

  while read -r child_id; do
    [ -n "$child_id" ] && terminate_process_tree "$child_id"
  done < <(pgrep -P "$process_id" 2>/dev/null || true)
  kill -TERM "$process_id" 2>/dev/null || true
}

cleanup() {
  local status=$?
  trap - EXIT INT TERM
  set +e

  for process_id in "${PROCESS_IDS[@]}"; do
    terminate_process_tree "$process_id"
  done
  for process_id in "${PROCESS_IDS[@]}"; do
    wait "$process_id" 2>/dev/null
  done

  if [ "$status" -ne 0 ]; then
    if [ -n "$ARTIFACT_DIR" ]; then
      mkdir -p "$ARTIFACT_DIR"
      for log_file in "$RUNTIME_DIR"/*.log; do
        [ -f "$log_file" ] || continue
        cp "$log_file" "$ARTIFACT_DIR/"
      done
      compose logs --no-color >"$ARTIFACT_DIR/compose.log" 2>&1
    fi

    for log_file in "$RUNTIME_DIR"/*.log; do
      [ -f "$log_file" ] || continue
      printf '\n==> %s\n' "$(basename "$log_file")" >&2
      tail -n 120 "$log_file" >&2
    done
    compose logs --no-color --tail=120 >&2
  fi

  compose down --volumes --remove-orphans >/dev/null 2>&1
  rm -rf -- "$RUNTIME_DIR"
  exit "$status"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

wait_for_http() {
  local name="$1"
  local url="$2"
  local process_id="${3:-}"

  for _ in $(seq 1 300); do
    if [ -n "$process_id" ] && ! kill -0 "$process_id" 2>/dev/null; then
      fail "$name exited before becoming ready"
    fi
    if curl --fail --silent --show-error --max-time 2 "$url" >/dev/null 2>&1; then
      return 0
    fi
    sleep 1
  done
  fail "$name did not become ready at $url"
}

start_process() {
  local name="$1"
  shift
  "$@" >"$RUNTIME_DIR/$name.log" 2>&1 &
  LAST_PROCESS_ID="$!"
  PROCESS_IDS+=("$LAST_PROCESS_ID")
}

export NVBES_TARGET_ENV=local
export NVBES_ENV=development
export NVBES_ALLOW_DESTRUCTIVE_TEST_DATABASE=account-quality-v1
export NVBES_DATABASE_URL="postgres://postgres:postgres@127.0.0.1:$POSTGRES_PORT/nvbes_identity_test_e2e"
export NVBES_REDIS_URL="redis://127.0.0.1:$REDIS_PORT/15"
export NVBES_REDIS_PASSWORD=
export NVBES_SECRET_MANAGER_ENABLED=false
export NVBES_ENTERPRISE_GRPC_ENDPOINT=
export NVBES_ENTERPRISE_GRPC_AUTH_TOKEN=
export NVBES_CLOUD_GRPC_ENDPOINT="http://127.0.0.1:$CLOUD_GRPC_PORT"
export NVBES_EMAIL_PROVIDER=test-capture
export NVBES_EMAIL_TEST_CAPTURE_DIR="$EMAIL_CAPTURE_DIR"
export NVBES_BETA_SEED_EMAIL="identity-e2e@example.test"
export NVBES_BETA_SEED_PASSWORD="IdentityE2e-Strong-Password-2026!"
export NVBES_BETA_SEED_WORKSPACE="Identity E2E Workspace"
export NVBES_API_BASE_URL="http://localhost:$IDENTITY_HTTP_PORT"
export NVBES_WEB_BASE_URL="http://localhost:$IDENTITY_WEB_PORT"
export NVBES_IDENTITY_SERVICE_BASE_URL="$NVBES_API_BASE_URL"
export NVBES_IDENTITY_GRPC_ENDPOINT="http://127.0.0.1:$IDENTITY_GRPC_PORT"
export NVBES_IDENTITY_GRPC_PORT="$IDENTITY_GRPC_PORT"
export NVBES_JWT_SECRET="identity-e2e-jwt-secret-at-least-32-bytes-long"
export NVBES_AUTH_FACTOR_ENCRYPTION_KEY="AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA="
export NVBES_AUTH_FACTOR_ENCRYPTION_KEY_VERSION=1
export NVBES_AUTH_POW_ENABLED=true
export NVBES_AUTH_POW_DIFFICULTY=8
export NVBES_OTP_PROVIDER=mock
export NVBES_WEBAUTHN_RP_ID=localhost
export NVBES_WEBAUTHN_RP_ORIGIN="$NVBES_WEB_BASE_URL"
export NVBES_ADDITIONAL_CORS_ORIGINS="$NVBES_WEB_BASE_URL"
export NVBES_ACCOUNT_WEB_OAUTH_REDIRECT_URIS="$NVBES_WEB_BASE_URL/oauth/callback"
export NVBES_PRODUCT_ANALYTICS_ENABLED=false
export NVBES_REQUEST_E2EE_ENABLED=false
export NVBES_REQUEST_E2EE_REQUIRED=false
export VITE_IDENTITY_SERVICE_BASE_URL="$NVBES_API_BASE_URL"
export VITE_IDENTITY_WEB_PORT="$IDENTITY_WEB_PORT"

log_step "starting isolated PostgreSQL and Redis"
compose up --detach --wait

log_step "starting Cloud Service"
start_process cloud-service env \
  NVBES_APP_NAME=nvbes-cloud-e2e \
  NVBES_DATABASE_URL="postgres://postgres:postgres@127.0.0.1:$POSTGRES_PORT/nvbes_cloud_test_e2e" \
  NVBES_CLOUD_RUNTIME_DATABASE_URL="postgres://postgres:postgres@127.0.0.1:$POSTGRES_PORT/nvbes_cloud_test_e2e" \
  NVBES_CLOUD_SYSTEM_DATABASE_URL="postgres://postgres:postgres@127.0.0.1:$POSTGRES_PORT/nvbes_cloud_test_e2e" \
  NVBES_API_PORT="$CLOUD_HTTP_PORT" \
  NVBES_CLOUD_GRPC_PORT="$CLOUD_GRPC_PORT" \
  STORAGE_ENABLED=false \
  SCAN_ENABLED=false \
  cargo run --quiet --package nvbes-cloud-service
CLOUD_PROCESS_ID="$LAST_PROCESS_ID"
wait_for_http "Cloud Service" "http://127.0.0.1:$CLOUD_HTTP_PORT/health" "$CLOUD_PROCESS_ID"

log_step "starting Identity Service"
start_process identity-service env \
  NVBES_APP_NAME=nvbes-identity-e2e \
  NVBES_API_PORT="$IDENTITY_HTTP_PORT" \
  cargo run --quiet --package nvbes-identity-service
IDENTITY_PROCESS_ID="$LAST_PROCESS_ID"
wait_for_http "Identity Service" "$NVBES_API_BASE_URL/health" "$IDENTITY_PROCESS_ID"

log_step "starting Identity Worker"
start_process identity-worker env \
  NVBES_APP_NAME=nvbes-identity-worker-e2e \
  NVBES_IDENTITY_WORKER_INLINE_HOUSEKEEPING_ENABLED=false \
  NVBES_IDENTITY_WORKER_METRICS_BIND_ADDR="127.0.0.1:$WORKER_METRICS_PORT" \
  cargo run --quiet --package nvbes-identity-worker
IDENTITY_WORKER_PROCESS_ID="$LAST_PROCESS_ID"
wait_for_http \
  "Identity Worker" \
  "http://127.0.0.1:$WORKER_METRICS_PORT/metrics" \
  "$IDENTITY_WORKER_PROCESS_ID"

log_step "starting Identity Web"
start_process identity-web pnpm --dir apps/identity-web dev
IDENTITY_WEB_PROCESS_ID="$LAST_PROCESS_ID"
wait_for_http "Identity Web" "$NVBES_WEB_BASE_URL/login" "$IDENTITY_WEB_PROCESS_ID"

log_step "running critical Identity browser journeys"
bash "$ROOT_DIR/scripts/test-e2e-critical.sh"
