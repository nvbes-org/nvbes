#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib/test-env.sh"

require_cmd curl
require_cmd node
require_env NVBES_WEB_BASE_URL
require_env NVBES_API_BASE_URL

WEB_BASE_URL="$(normalize_url "$NVBES_WEB_BASE_URL")"
API_BASE_URL="$(normalize_url "$NVBES_API_BASE_URL")"
WEB_MARKER="${NVBES_SMOKE_WEB_MARKER:-nvbes}"

log_step "web is reachable"
web_body="$(http_body "$WEB_BASE_URL")"
assert_contains "$web_body" "$WEB_MARKER"

log_step "api health"
health_body="$(http_body "$API_BASE_URL/health")"
assert_contains "$health_body" "\"status\":\"ok\""

log_step "request id propagation"
assert_header_contains "$API_BASE_URL/health" "x-request-id"

log_step "metrics endpoint is not public"
metrics_status="$(http_status "$API_BASE_URL/metrics")"
case "$metrics_status" in
  401 | 403) ;;
  *) fail "expected protected metrics endpoint to return 401 or 403, got $metrics_status" ;;
esac

log_step "public share route is mounted"
assert_http_not_5xx "$API_BASE_URL/public/shares/smoke-invalid-token"

# Run seeded auth FIRST when enabled, so it gets a clean server state
# (the public-only probe exhausts per-IP counters or rate-limit entries,
# which causes UND_ERR_SOCKET on seeded auth's PoW endpoint).
if [ "${NVBES_SMOKE_INCLUDE_SEEDED_AUTH:-}" = "1" ] || [ "${NVBES_SMOKE_INCLUDE_SEEDED_AUTH:-}" = "true" ]; then
  log_step "openapi contract smoke (seeded auth)"
  bash "$SCRIPT_DIR/test-openapi-contract-seeded-auth.sh"
fi

log_step "openapi contract smoke"
bash "$SCRIPT_DIR/test-openapi-contract.sh"
