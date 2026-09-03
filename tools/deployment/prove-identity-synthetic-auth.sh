#!/usr/bin/env bash
set -euo pipefail

: "${STACK_ROOT:?STACK_ROOT is required}"
: "${SCW_SECRET_KEY:?SCW_SECRET_KEY is required}"

job_ref="$(terraform -chdir="$STACK_ROOT" output -raw identity_synthetic_auth_job_id)"
job_id="${job_ref##*/}"
synthetic_email="identity-${GITHUB_RUN_ID:?}-${GITHUB_RUN_ATTEMPT:?}@synthetic.invalid"
payload="$(jq --null-input --arg email "$synthetic_email" \
  '{environment_variables: {NVBES_IDENTITY_SYNTHETIC_EMAIL: $email}}')"
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
  psql "$runtime_url" --no-psqlrc --tuples-only --no-align \
    --set ON_ERROR_STOP=1 --set synthetic_email="$synthetic_email" \
    --file=- <<'SQL'
    SELECT json_build_object(
      'principal_id', p.id,
      'audit_events', count(DISTINCT a.id),
      'sessions', count(DISTINCT s.id),
      'recovery_consumed', bool_or(r.consumed_at IS NOT NULL),
      'old_sessions_revoked', bool_and(s.revoked_at IS NOT NULL) FILTER (WHERE s.created_at < (SELECT max(created_at) FROM identity_sessions WHERE principal_id = p.id))
    )
    FROM identity_principals p
    JOIN identity_login_identifiers i ON i.principal_id = p.id
    JOIN identity_audit_events a ON a.principal_id = p.id
    JOIN identity_sessions s ON s.principal_id = p.id
    JOIN identity_recovery_challenges r ON r.principal_id = p.id
    WHERE i.kind = 'email' AND i.normalized_value = :'synthetic_email'
    GROUP BY p.id;
SQL
)"
jq --exit-status \
  '.principal_id != null and .audit_events == 5 and .sessions == 2 and .recovery_consumed == true and .old_sessions_revoked == true' \
  <<<"$evidence" >/dev/null
printf '### Identity synthetic authentication\n\n```json\n%s\n```\n' "$evidence" >> "$GITHUB_STEP_SUMMARY"
