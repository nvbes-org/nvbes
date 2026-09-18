#!/usr/bin/env bash
set -euo pipefail

: "${STACK_ROOT:?STACK_ROOT is required}"
: "${SCW_SECRET_KEY:?SCW_SECRET_KEY is required}"
: "${GITHUB_RUN_ID:?GITHUB_RUN_ID is required}"
: "${GITHUB_RUN_ATTEMPT:?GITHUB_RUN_ATTEMPT is required}"

job_ref="$(terraform -chdir="$STACK_ROOT" output -raw identity_synthetic_invitation_job_id)"
job_id="${job_ref##*/}"
inviter_email="identity-inviter-${GITHUB_RUN_ID}-${GITHUB_RUN_ATTEMPT}@synthetic.invalid"
invited_email="identity-invited-${GITHUB_RUN_ID}-${GITHUB_RUN_ATTEMPT}@synthetic.invalid"
payload="$(jq --null-input --arg inviter "$inviter_email" --arg invited "$invited_email" \
  '{environment_variables: {
    NVBES_IDENTITY_SYNTHETIC_INVITER_EMAIL: $inviter,
    NVBES_IDENTITY_SYNTHETIC_INVITED_EMAIL: $invited
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
    --set invited_email="$invited_email" --file=- <<'SQL'
SELECT json_build_object(
  'accepted', invitation.state = 'accepted',
  'code_hash_bytes', octet_length(invitation.code_hash),
  'accepted_principal_matches', invitation.accepted_principal_id = identifier.principal_id
)
FROM identity_invitations invitation
JOIN identity_login_identifiers identifier
  ON identifier.principal_id = invitation.accepted_principal_id
WHERE identifier.kind = 'email' AND identifier.normalized_value = :'invited_email'
ORDER BY invitation.accepted_at DESC
LIMIT 1;
SQL
)"
jq --exit-status \
  '.accepted == true and .code_hash_bytes == 32 and
   .accepted_principal_matches == true' \
  <<<"$evidence" >/dev/null
printf '### Identity invitation proof\n\n```json\n%s\n```\n' "$evidence" >> "$GITHUB_STEP_SUMMARY"
