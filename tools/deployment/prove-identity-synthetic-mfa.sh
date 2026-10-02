#!/usr/bin/env bash
set -euo pipefail

: "${STACK_ROOT:?STACK_ROOT is required}"
: "${SCW_SECRET_KEY:?SCW_SECRET_KEY is required}"
: "${GITHUB_RUN_ID:?GITHUB_RUN_ID is required}"
: "${GITHUB_RUN_ATTEMPT:?GITHUB_RUN_ATTEMPT is required}"

job_ref="$(terraform -chdir="$STACK_ROOT" output -raw identity_synthetic_mfa_job_id)"
job_id="${job_ref##*/}"
synthetic_email="identity-mfa-${GITHUB_RUN_ID}-${GITHUB_RUN_ATTEMPT}@synthetic.invalid"
mfa_key_version="$(terraform -chdir="$STACK_ROOT" output -raw identity_mfa_key_version)"
payload="$(jq --null-input --arg email "$synthetic_email" --arg mfa_key_version "$mfa_key_version" \
  '{environment_variables: {
    NVBES_ENVIRONMENT: "production",
    NVBES_IDENTITY_SYNTHETIC_EMAIL: $email,
    NVBES_IDENTITY_MFA_KEY_VERSION: $mfa_key_version
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
SELECT json_build_object(
  'factor_active', f.state = 'active',
  'factor_encrypted', octet_length(f.secret_ciphertext) > 0,
  'step_up_granted', s.step_up_expires_at > clock_timestamp(),
  'step_up_method', s.step_up_method,
  'step_up_audited', EXISTS (
    SELECT 1 FROM identity_audit_events a
    WHERE a.principal_id = p.id AND a.event_type = 'identity.step_up_granted'
  )
)
FROM identity_principals p
JOIN identity_login_identifiers i ON i.principal_id = p.id
JOIN identity_auth_factors f ON f.principal_id = p.id
JOIN identity_sessions s ON s.principal_id = p.id
WHERE i.kind = 'email' AND i.normalized_value = :'synthetic_email'
ORDER BY s.created_at DESC
LIMIT 1;
SQL
)"
jq --exit-status \
  '.factor_active == true and .factor_encrypted == true and
   .step_up_granted == true and .step_up_method == "totp" and .step_up_audited == true' \
  <<<"$evidence" >/dev/null
printf '### Identity MFA proof\n\n```json\n%s\n```\n' "$evidence" >> "$GITHUB_STEP_SUMMARY"
