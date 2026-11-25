#!/usr/bin/env bash
set -euo pipefail

: "${STACK_ROOT:?STACK_ROOT is required}"
: "${BILLING_METRICS_TOKEN:?BILLING_METRICS_TOKEN is required}"
: "${GITHUB_STEP_SUMMARY:?GITHUB_STEP_SUMMARY is required}"

endpoint="$(terraform -chdir="$STACK_ROOT" output -raw billing_endpoint)"
SECONDS=0
liveness="$(curl --fail --silent --show-error --max-time 60 \
  --retry 12 --retry-all-errors --retry-delay 5 "${endpoint}/health/live")"
activation_seconds="$SECONDS"
jq --exit-status \
  '.status == "alive" and .service == "nvbes-billing-service"' \
  <<<"$liveness" >/dev/null

readiness="$(curl --fail --silent --show-error --max-time 60 \
  --retry 12 --retry-all-errors --retry-delay 5 "${endpoint}/health/ready")"
jq --exit-status \
  '.status == "ready" and .service == "nvbes-billing-service"' \
  <<<"$readiness" >/dev/null

curl --fail --silent --show-error \
  --header "Authorization: Bearer ${BILLING_METRICS_TOKEN}" \
  "${endpoint}/metrics" | grep -q 'http_requests_total'

anonymous_metrics_status="$(curl --silent --output /dev/null --write-out '%{http_code}' \
  "${endpoint}/metrics")"
[[ "$anonymous_metrics_status" == "401" ]]

registration_status="$(curl --silent --output /dev/null --write-out '%{http_code}' \
  --request POST "${endpoint}/auth/register")"
[[ "$registration_status" == "404" ]]

evidence="$(jq --null-input \
  --argjson activation_seconds "$activation_seconds" \
  --argjson anonymous_metrics_status "$anonymous_metrics_status" \
  --argjson registration_status "$registration_status" \
  '{activation_seconds: $activation_seconds,
    liveness: "alive",
    readiness: "ready",
    authenticated_metrics: true,
    anonymous_metrics_status: $anonymous_metrics_status,
    public_registration_status: $registration_status}')"
printf '### Billing public runtime\n\n```json\n%s\n```\n' "$evidence" >> "$GITHUB_STEP_SUMMARY"
