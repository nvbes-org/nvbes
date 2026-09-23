#!/usr/bin/env bash
set -euo pipefail

: "${STACK_ROOT:?STACK_ROOT is required}"
: "${IDENTITY_METRICS_TOKEN:?IDENTITY_METRICS_TOKEN is required}"
: "${GITHUB_STEP_SUMMARY:?GITHUB_STEP_SUMMARY is required}"

endpoint="$(terraform -chdir="$STACK_ROOT" output -raw identity_endpoint)"
SECONDS=0
liveness="$(curl --fail --silent --show-error --max-time 60 \
  --retry 12 --retry-all-errors --retry-delay 5 "${endpoint}/health/live")"
activation_seconds="$SECONDS"
jq --exit-status \
  '.status == "alive" and .service == "nvbes-identity-service"' \
  <<<"$liveness" >/dev/null
readiness="$(curl --fail --silent --show-error --max-time 60 \
  --retry 12 --retry-all-errors --retry-delay 5 "${endpoint}/health/ready")"
jq --exit-status \
  '.status == "ready" and .service == "nvbes-identity-service"' \
  <<<"$readiness" >/dev/null
curl --fail --silent --show-error \
  --header "Authorization: Bearer ${IDENTITY_METRICS_TOKEN}" \
  "${endpoint}/metrics" | grep -q 'http_requests_total'
anonymous_metrics_status="$(curl --silent --output /dev/null --write-out '%{http_code}' \
  "${endpoint}/metrics")"
[[ "$anonymous_metrics_status" == "401" ]]
registration_status="$(curl --silent --output /dev/null --write-out '%{http_code}' \
  --request POST \
  --header 'content-type: application/json' \
  --data '{"email":"closed@synthetic.invalid","password":"password-123456"}' \
  "${endpoint}/api/v1/auth/register")"
[[ "$registration_status" == "403" ]]
login_status="$(curl --silent --output /dev/null --write-out '%{http_code}' \
  --request POST "${endpoint}/api/v1/auth/login")"
[[ "$login_status" == "415" ]]
authorize_status="$(curl --silent --output /dev/null --write-out '%{http_code}' \
  "${endpoint}/oauth/authorize?response_type=code&client_id=probe&redirect_uri=https://example.com/cb")"
[[ "$authorize_status" == "401" ]]

evidence="$(jq --null-input \
  --argjson activation_seconds "$activation_seconds" \
  --argjson anonymous_metrics_status "$anonymous_metrics_status" \
  --argjson registration_status "$registration_status" \
  --argjson login_status "$login_status" \
  --argjson authorize_status "$authorize_status" \
  '{activation_seconds: $activation_seconds,
    liveness: "alive",
    readiness: "ready",
    authenticated_metrics: true,
    anonymous_metrics_status: $anonymous_metrics_status,
    public_registration_status: $registration_status,
    login_probe_status: $login_status,
    authorize_probe_status: $authorize_status}')"
printf '### Identity public runtime\n\n```json\n%s\n```\n' "$evidence" >> "$GITHUB_STEP_SUMMARY"
