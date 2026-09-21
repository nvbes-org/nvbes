#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

# shellcheck disable=SC1091
source "$ROOT_DIR/scripts/lib/test-env.sh"

require_cmd cargo
require_cmd curl
require_cmd node
export DATABASE_URL="${DATABASE_URL:-${NVBES_EMAIL_DATABASE_URL:-}}"
require_env DATABASE_URL
node "$ROOT_DIR/scripts/validate-email-test-database.mjs"

free_port() {
  node --input-type=module -e '
    import net from "node:net";
    const server = net.createServer();
    server.listen(0, "127.0.0.1", () => {
      process.stdout.write(String(server.address().port));
      server.close();
    });
  '
}

EMAIL_HTTP_PORT="$(free_port)"
EMAIL_E2E_DIR="$(mktemp -d)"
EMAIL_WORKER_PID=""

cleanup() {
  if [[ -n "$EMAIL_WORKER_PID" ]]; then
    kill -TERM "$EMAIL_WORKER_PID" 2>/dev/null || true
    wait "$EMAIL_WORKER_PID" 2>/dev/null || true
  fi
  rm -rf "$EMAIL_E2E_DIR"
}
trap cleanup EXIT

cargo build --locked --package nvbes-email-worker

NVBES_ENVIRONMENT=test \
NVBES_EMAIL_DATABASE_URL="$DATABASE_URL" \
NVBES_EMAIL_HTTP_BIND_ADDR="127.0.0.1:$EMAIL_HTTP_PORT" \
NVBES_EMAIL_FROM_EMAIL="no-reply@nvbes.fr" \
NVBES_EMAIL_PROVIDER=test-capture \
NVBES_EMAIL_TEST_CAPTURE_DIR="$EMAIL_E2E_DIR/captures" \
NVBES_EMAIL_PRODUCER_TOKENS="identity-service=email-worker-internal-token-32-value,backoffice-service=backoffice-email-internal-token-32-val" \
  "$ROOT_DIR/target/debug/nvbes-email-worker" migrate

NVBES_ENVIRONMENT=test \
NVBES_EMAIL_DATABASE_URL="$DATABASE_URL" \
NVBES_EMAIL_HTTP_BIND_ADDR="127.0.0.1:$EMAIL_HTTP_PORT" \
NVBES_EMAIL_FROM_EMAIL="no-reply@nvbes.fr" \
NVBES_EMAIL_PROVIDER=test-capture \
NVBES_EMAIL_TEST_CAPTURE_DIR="$EMAIL_E2E_DIR/captures" \
NVBES_EMAIL_PRODUCER_TOKENS="identity-service=email-worker-internal-token-32-value,backoffice-service=backoffice-email-internal-token-32-val" \
  "$ROOT_DIR/target/debug/nvbes-email-worker" >"$EMAIL_E2E_DIR/worker.log" 2>&1 &
EMAIL_WORKER_PID=$!

EMAIL_BASE_URL="http://127.0.0.1:$EMAIL_HTTP_PORT"
for _ in $(seq 1 100); do
  if curl --fail --silent --show-error "$EMAIL_BASE_URL/health/live" >/dev/null 2>&1; then
    break
  fi
  if ! kill -0 "$EMAIL_WORKER_PID" 2>/dev/null; then
    sed -n '1,200p' "$EMAIL_E2E_DIR/worker.log" >&2
    fail "email worker exited before becoming live"
  fi
  sleep 0.1
done

assert_http_status "$EMAIL_BASE_URL/health/live" 200
assert_http_status "$EMAIL_BASE_URL/health/ready" 200
assert_http_status "$EMAIL_BASE_URL/metrics" 200

WEBHOOK_URL="$EMAIL_BASE_URL/webhooks/scaleway/topics-and-events"
WEBHOOK_STATUS="$(curl --silent --show-error --output /dev/null --write-out '%{http_code}' \
  --request POST --data '{}' "$WEBHOOK_URL")"
[[ "$WEBHOOK_STATUS" = "415" ]] || fail "expected webhook HTTP 415, got $WEBHOOK_STATUS"
WEBHOOK_STATUS="$(curl --silent --show-error --output /dev/null --write-out '%{http_code}' \
  --request POST --header 'content-type: application/json' --data '{}' "$WEBHOOK_URL")"
[[ "$WEBHOOK_STATUS" = "503" ]] || fail "expected disabled webhook HTTP 503, got $WEBHOOK_STATUS"

kill -TERM "$EMAIL_WORKER_PID"
wait "$EMAIL_WORKER_PID"
EMAIL_WORKER_PID=""
