#!/usr/bin/env bash
set -euo pipefail

: "${STACK_ROOT:?STACK_ROOT is required}"
: "${SCW_SECRET_KEY:?SCW_SECRET_KEY is required}"
: "${GITHUB_STEP_SUMMARY:?GITHUB_STEP_SUMMARY is required}"

job_ref="$(terraform -chdir="$STACK_ROOT" output -raw account_synthetic_smoke_job_id)"
job_id="${job_ref##*/}"

owner_id="$(uuidgen | tr '[:upper:]' '[:lower:]')"
member_id="$(uuidgen | tr '[:upper:]' '[:lower:]')"

payload="$(jq --null-input \
  --arg owner "$owner_id" \
  --arg member "$member_id" \
  '{environment_variables: {
    NVBES_ACCOUNT_SYNTHETIC_OWNER_ID: $owner,
    NVBES_ACCOUNT_SYNTHETIC_MEMBER_ID: $member
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

runtime_url="$(terraform -chdir="$STACK_ROOT" output -raw account_database_runtime_url)&sslrootcert=system"
evidence="$(docker run --rm --interactive --platform linux/amd64 postgres:17-alpine \
  psql "$runtime_url" --no-psqlrc --tuples-only --no-align \
  --set ON_ERROR_STOP=1 --set owner_id="$owner_id" --set member_id="$member_id" \
  --file=- <<'SQL'
  SELECT json_build_object(
    'owner_profile', (SELECT count(*) FROM account_profiles WHERE principal_id = :'owner_id'::uuid),
    'member_profile', (SELECT count(*) FROM account_profiles WHERE principal_id = :'member_id'::uuid),
    'teams', (SELECT count(*) FROM account_teams WHERE owner_principal_id = :'owner_id'::uuid),
    'export_completed', (SELECT bool_or(status = 'completed' AND document IS NOT NULL) FROM account_exports WHERE principal_id = :'owner_id'::uuid),
    'closure_cancelled', (SELECT bool_or(status = 'cancelled') FROM account_closures WHERE principal_id = :'member_id'::uuid),
    'audit_events', (SELECT count(*) FROM account_audit_events WHERE principal_id IN (:'owner_id'::uuid, :'member_id'::uuid)),
    'outbox_events', (SELECT count(*) FROM account_outbox WHERE aggregate_id = :'owner_id'::uuid)
  );
SQL
)"

jq --exit-status \
  '.owner_profile == 1 and .member_profile == 1 and .teams >= 1 and .export_completed == true and .closure_cancelled == true and .audit_events >= 4' \
  <<<"$evidence" >/dev/null

printf '### Account synthetic smoke proof\n\n```json\n%s\n```\n' "$evidence" >> "$GITHUB_STEP_SUMMARY"
