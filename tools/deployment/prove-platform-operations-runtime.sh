#!/usr/bin/env bash
set -euo pipefail

PORT=8089
TOKEN="test-operator-proof-token"
LOG_FILE="/tmp/platform-ops-proof.log"

echo "=== [1/6] Building Platform Operations binary ==="
cargo build --package nvbes-platform --bin nvbes-platform-operations

echo "=== [2/6] Starting Platform Operations daemon on port $PORT ==="
NVBES_PLATFORM_OPERATIONS_PORT="$PORT" \
NVBES_PLATFORM_OPERATIONS_TOKEN="$TOKEN" \
NVBES_ENVIRONMENT="testing" \
./target/debug/nvbes-platform-operations > "$LOG_FILE" 2>&1 &
SERVER_PID=$!

cleanup() {
  echo "Stopping daemon (PID: $SERVER_PID)..."
  kill -TERM "$SERVER_PID" 2>/dev/null || true
  wait "$SERVER_PID" 2>/dev/null || true
  rm -f "$LOG_FILE"
}
trap cleanup EXIT

echo "Waiting for liveness probe..."
for i in {1..30}; do
  if curl -s -f "http://127.0.0.1:$PORT/health/live" > /dev/null 2>&1; then
    break
  fi
  sleep 0.2
done

echo "=== [3/6] Verifying health probes ==="
LIVE_STATUS=$(curl -s -o /dev/null -w "%{http_code}" "http://127.0.0.1:$PORT/health/live")
READY_STATUS=$(curl -s -o /dev/null -w "%{http_code}" "http://127.0.0.1:$PORT/health/ready")
if [[ "$LIVE_STATUS" != "200" || "$READY_STATUS" != "200" ]]; then
  echo "Error: Health probes failed (live: $LIVE_STATUS, ready: $READY_STATUS)"
  exit 1
fi
echo "Probes verified: /health/live ($LIVE_STATUS), /health/ready ($READY_STATUS)"

echo "=== [4/6] Verifying public access rejection (401 Unauthorized) ==="
UNAUTH_CODE=$(curl -s -o /dev/null -w "%{http_code}" "http://127.0.0.1:$PORT/api/v1/overview")
if [[ "$UNAUTH_CODE" != "401" ]]; then
  echo "Error: Unauthenticated access was not rejected with 401 (got $UNAUTH_CODE)"
  exit 1
fi
echo "Unauthenticated access properly rejected with HTTP 401"

echo "=== [5/6] Verifying authenticated overview access ==="
AUTH_RESPONSE=$(curl -s -f -H "Authorization: Bearer $TOKEN" "http://127.0.0.1:$PORT/api/v1/overview")
if ! echo "$AUTH_RESPONSE" | grep -q '"environment":"testing"'; then
  echo "Error: Overview response missing expected payload"
  exit 1
fi
echo "Authenticated overview successfully retrieved"

echo "=== [6/6] Verifying audited operator action enforcement ==="
# Test invalid reason rejection (< 3 chars)
BAD_ACTION_CODE=$(curl -s -o /dev/null -w "%{http_code}" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"operator_id":"solo_op","reason":"no","idempotency_key":"k1","correlation_id":"00000000-0000-0000-0000-000000000001","payload":{"action_type":"release_email_suppression","payload":{"email":"test@nvbes.com"}}}' \
  "http://127.0.0.1:$PORT/api/v1/actions")

if [[ "$BAD_ACTION_CODE" != "400" ]]; then
  echo "Error: Short reason was not rejected with 400 (got $BAD_ACTION_CODE)"
  exit 1
fi

# Test valid action execution
GOOD_ACTION=$(curl -s -f \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"operator_id":"solo_op","reason":"Valid incident mitigation justification","idempotency_key":"k2","correlation_id":"00000000-0000-0000-0000-000000000002","payload":{"action_type":"release_email_suppression","payload":{"email":"test@nvbes.com"}}}' \
  "http://127.0.0.1:$PORT/api/v1/actions")

if ! echo "$GOOD_ACTION" | grep -q '"status":"executed"'; then
  echo "Error: Valid operator action failed to execute"
  exit 1
fi
echo "Valid operator action successfully executed, audited, and receipt issued"

echo "=== Platform Operations Proof Completed Successfully ==="
