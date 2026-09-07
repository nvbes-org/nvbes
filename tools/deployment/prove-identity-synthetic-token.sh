#!/usr/bin/env bash
set -euo pipefail

: "${STACK_ROOT:?STACK_ROOT is required}"
: "${SCW_SECRET_KEY:?SCW_SECRET_KEY is required}"
: "${GITHUB_RUN_ID:?GITHUB_RUN_ID is required}"
: "${GITHUB_RUN_ATTEMPT:?GITHUB_RUN_ATTEMPT is required}"

job_ref="$(terraform -chdir="$STACK_ROOT" output -raw identity_synthetic_token_job_id)"
job_id="${job_ref##*/}"
synthetic_email="identity-token-${GITHUB_RUN_ID}-${GITHUB_RUN_ATTEMPT}@synthetic.invalid"
issuer="$(terraform -chdir="$STACK_ROOT" output -raw identity_token_issuer)"
key_id="$(terraform -chdir="$STACK_ROOT" output -raw identity_token_key_id)"
audiences="$(terraform -chdir="$STACK_ROOT" output -raw identity_token_audiences)"
payload="$(jq --null-input --arg email "$synthetic_email" --arg issuer "$issuer" --arg key_id "$key_id" --arg audiences "$audiences" \
  '{environment_variables: {
    NVBES_ENVIRONMENT: "production",
    NVBES_IDENTITY_SYNTHETIC_EMAIL: $email,
    NVBES_IDENTITY_SYNTHETIC_TOKEN_AUDIENCE: "nvbes-account-service",
    NVBES_IDENTITY_TOKEN_ISSUER: $issuer,
    NVBES_IDENTITY_TOKEN_KEY_ID: $key_id,
    NVBES_IDENTITY_TOKEN_AUDIENCES: $audiences
  }}')"
response="$(curl --fail-with-body --silent --show-error --request POST \
  --header "X-Auth-Token: $SCW_SECRET_KEY" --header 'Content-Type: application/json' \
  --data "$payload" \
  "https://api.scaleway.com/serverless-jobs/v1alpha2/regions/fr-par/job-definitions/${job_id}/start")"
run_id="$(jq -er '.job_runs[0].id' <<<"$response")"
state=queued
for _attempt in {1..60}; do
  response="$(curl --fail-with-body --silent --show-error \
    --header "X-Auth-Token: $SCW_SECRET_KEY" \
    "https://api.scaleway.com/serverless-jobs/v1alpha2/regions/fr-par/job-runs/${run_id}")"
  state="$(jq -er '.state' <<<"$response")"
  case "$state" in
    succeeded) break ;;
    failed|canceled|interrupted) jq -r '.reason // "unknown"' <<<"$response" >&2; exit 1 ;;
  esac
  sleep 10
done
[[ "$state" == "succeeded" ]]

runtime_url="$(terraform -chdir="$STACK_ROOT" output -raw identity_database_runtime_url)&sslrootcert=system"
evidence="$(docker run --rm --interactive --platform linux/amd64 postgres:16-alpine \
  psql "$runtime_url" --no-psqlrc --tuples-only --no-align --set ON_ERROR_STOP=1 \
    --set synthetic_email="$synthetic_email" --file=- <<'SQL'
SELECT a.details
FROM identity_audit_events a
JOIN identity_login_identifiers i ON i.principal_id = a.principal_id
WHERE a.event_type = 'identity.token.synthetic_proven'
  AND i.kind = 'email'
  AND i.normalized_value = :'synthetic_email'
ORDER BY a.occurred_at DESC
LIMIT 1;
SQL
)"
jq --exit-status \
  '.audience == "nvbes-account-service" and
   .scope == "account:read account:write account:export account:close" and
   .amr == ["pwd"] and
   .active_before_revocation == true and
   .inactive_after_revocation == true' \
  <<<"$evidence" >/dev/null
printf '### Identity Account-token proof\n\n```json\n%s\n```\n' "$evidence" >> "$GITHUB_STEP_SUMMARY"
