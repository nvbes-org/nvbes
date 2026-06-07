#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib/test-env.sh"

require_cmd curl
require_cmd jq
require_cmd cargo
require_cmd node
require_env NVBES_DATABASE_URL
require_env NVBES_BETA_SEED_EMAIL
require_env NVBES_BETA_SEED_PASSWORD

if [ -z "${NVBES_API_BASE_URL:-}" ] \
  && [ -z "${NVBES_IDENTITY_API_BASE_URL:-${NVBES_STAGING_IDENTITY_API_BASE_URL:-}}" ]; then
  fail "missing required environment variable: NVBES_API_BASE_URL or NVBES_IDENTITY_API_BASE_URL"
fi
if [ -z "${NVBES_API_BASE_URL:-}" ] \
  && [ -z "${NVBES_DRIVE_API_BASE_URL:-${NVBES_STAGING_DRIVE_API_BASE_URL:-}}" ]; then
  fail "missing required environment variable: NVBES_API_BASE_URL or NVBES_DRIVE_API_BASE_URL"
fi

IDENTITY_API_BASE_URL="$(normalize_url "${NVBES_IDENTITY_API_BASE_URL:-${NVBES_STAGING_IDENTITY_API_BASE_URL:-$NVBES_API_BASE_URL}}")"
DRIVE_API_BASE_URL="$(normalize_url "${NVBES_DRIVE_API_BASE_URL:-${NVBES_STAGING_DRIVE_API_BASE_URL:-$NVBES_API_BASE_URL}}")"
IDENTITY_API_ORIGIN="${IDENTITY_API_BASE_URL}"
SEED_WORKSPACE="${NVBES_BETA_SEED_WORKSPACE:-Beta Staging Workspace}"
SEED_FIRSTNAME="${NVBES_BETA_SEED_FIRSTNAME:-Beta}"
SEED_LASTNAME="${NVBES_BETA_SEED_LASTNAME:-Owner}"
SEED_USERNAME="${NVBES_BETA_SEED_USERNAME:-beta_$(date -u +%Y%m%d%H%M%S)}"
SEED_SUFFIX="${NVBES_BETA_SEED_SUFFIX:-$(date -u +%Y%m%d%H%M%S)}"
SEED_FILE_NAME="${NVBES_BETA_SEED_FILE_NAME:-beta-readiness-$SEED_SUFFIX.txt}"

json_post() {
  local path="$1"
  local body="$2"
  local token="${3:-}"

  if [ -n "$token" ]; then
    curl --fail --silent --show-error --location --max-time 20 \
      --header "content-type: application/json" \
      --header "authorization: Bearer $token" \
      --data "$body" \
      "$DRIVE_API_BASE_URL$path"
  else
    curl --fail --silent --show-error --location --max-time 20 \
      --header "content-type: application/json" \
      --header "origin: $IDENTITY_API_ORIGIN" \
      --data "$body" \
      "$IDENTITY_API_BASE_URL$path"
  fi
}

pow_body() {
  local challenge
  local nonce
  local difficulty
  local solution

  if ! challenge="$(curl --fail --silent --show-error --location --max-time 20 \
    --header "accept: application/json" \
    "$IDENTITY_API_BASE_URL/auth/challenge/pow" 2>/tmp/nvbes-beta-pow.err)"; then
    printf '{}'
    return 0
  fi

  nonce="$(printf '%s' "$challenge" | jq -r '.nonce // empty')"
  difficulty="$(printf '%s' "$challenge" | jq -r '.difficulty // 0')"
  if [ -z "$nonce" ] || [ "$difficulty" = "0" ]; then
    printf '{}'
    return 0
  fi

  solution="$(node - "$nonce" "$difficulty" <<'NODE'
const { createHash } = require("node:crypto");
const nonce = process.argv[2];
const difficulty = Number(process.argv[3]);

function leadingZeroBits(hash) {
  let count = 0;
  for (const byte of hash) {
    if (byte === 0) {
      count += 8;
    } else {
      count += Math.clz32(byte) - 24;
      break;
    }
  }
  return count;
}

for (let attempt = 0; attempt < 5_000_000; attempt += 1) {
  const candidate = String(attempt);
  const hash = createHash("sha256").update(`${nonce}:${candidate}`).digest();
  if (leadingZeroBits(hash) >= difficulty) {
    process.stdout.write(candidate);
    process.exit(0);
  }
}

process.stderr.write(`unable to solve PoW challenge for nonce ${nonce}\n`);
process.exit(1);
NODE
)"

  jq -n --arg nonce "$nonce" --arg solution "$solution" \
    '{pow_nonce:$nonce,pow_solution:$solution}'
}

log_step "register beta owner"
register_pow="$(pow_body)"
register_body="$(jq -n \
  --arg email "$NVBES_BETA_SEED_EMAIL" \
  --arg password "$NVBES_BETA_SEED_PASSWORD" \
  --arg firstname "$SEED_FIRSTNAME" \
  --arg lastname "$SEED_LASTNAME" \
  --arg username "$SEED_USERNAME" \
  --arg workspace "$SEED_WORKSPACE" \
  --argjson pow "$register_pow" \
  '{
    email:$email,
    password:$password,
    firstname:$firstname,
    lastname:$lastname,
    username:$username,
    region:"FR",
    workspace_name:$workspace
  } + $pow')"

if register_response="$(json_post "/auth/register" "$register_body" 2>/tmp/nvbes-beta-register.err)"; then
  :
else
  if grep -q "409" /tmp/nvbes-beta-register.err; then
    printf 'Account already exists, continuing with login.\n'
  else
    cat /tmp/nvbes-beta-register.err >&2
    exit 1
  fi
fi

log_step "prepare beta browser account"
NVBES_ENV=staging \
  NVBES_DATABASE_URL="$NVBES_DATABASE_URL" \
  cargo run -q -p nvbes-identity-api -- --prepare-beta-e2e-account \
    --email "$NVBES_BETA_SEED_EMAIL" \
    --password "$NVBES_BETA_SEED_PASSWORD" \
    --workspace-name "$SEED_WORKSPACE"

log_step "login beta owner"
identifier_pow="$(pow_body)"
identifier_body="$(jq -n \
  --arg email "$NVBES_BETA_SEED_EMAIL" \
  --argjson pow "$identifier_pow" \
  '{email:$email} + $pow')"
identifier_response="$(json_post "/auth/challenge/identifier" "$identifier_body")"
state_token="$(printf '%s' "$identifier_response" | jq -r '.state_token')"
if [ -z "$state_token" ] || [ "$state_token" = "null" ]; then
  fail "identifier challenge did not return state_token"
fi

login_body="$(jq -n \
  --arg state_token "$state_token" \
  --arg password "$NVBES_BETA_SEED_PASSWORD" \
  '{state_token:$state_token,password:$password}')"
login_response="$(json_post "/auth/challenge/pwd" "$login_body")"
session_token="$(printf '%s' "$login_response" | jq -r '.session_token')"
if [ -z "$session_token" ] || [ "$session_token" = "null" ]; then
  fail "password challenge did not return session_token; beta seed requires a non-MFA account"
fi

email_verified="$(printf '%s' "$login_response" | jq -r '.user.email_verified')"
if [ "$email_verified" != "true" ]; then
  log_step "extract verification token from queued email"
  verification_token="$(NVBES_ENV=staging \
    NVBES_DATABASE_URL="$NVBES_DATABASE_URL" \
    cargo run -q -p nvbes-identity-api -- --extract-email-token \
      --email "$NVBES_BETA_SEED_EMAIL" \
      --business-type verification)"

  log_step "verify beta owner email"
  verify_body="$(jq -n --arg token "$verification_token" '{token:$token}')"
  json_post "/auth/verify-email" "$verify_body" >/dev/null
fi

log_step "resolve beta workspace"
me_response="$(curl --fail --silent --show-error --location --max-time 20 \
  --header "authorization: Bearer $session_token" \
  "$IDENTITY_API_BASE_URL/auth/me")"
workspace_id="$(printf '%s' "$me_response" | jq -r '.current_workspace_id // empty')"
if [ -z "$workspace_id" ]; then
  workspace_response="$(curl --fail --silent --show-error --location --max-time 20 \
    --header "authorization: Bearer $session_token" \
    "$IDENTITY_API_BASE_URL/workspaces")"
  workspace_id="$(printf '%s' "$workspace_response" | jq -r '.workspaces[0].id // .items[0].id // empty')"
fi
if [ -z "$workspace_id" ]; then
  fail "could not resolve beta workspace id"
fi

log_step "create beta folder"
folder_body="$(jq -n --arg name "Beta validation $SEED_SUFFIX" '{name:$name}')"
folder_response="$(json_post "/workspaces/$workspace_id/folders" "$folder_body" "$session_token")"
folder_id="$(printf '%s' "$folder_response" | jq -r '.object.id')"

log_step "create and complete beta upload metadata"
upload_body="$(jq -n \
  --arg parent "$folder_id" \
  --arg name "$SEED_FILE_NAME" \
  '{parentId:$parent,name:$name,mimeType:"text/plain",expectedSizeBytes:128}')"
upload_response="$(json_post "/workspaces/$workspace_id/uploads" "$upload_body" "$session_token")"
upload_id="$(printf '%s' "$upload_response" | jq -r '.upload_id')"
complete_body="$(jq -n '{sizeBytes:128}')"
complete_response="$(json_post "/workspaces/$workspace_id/uploads/$upload_id/complete" "$complete_body" "$session_token")"
object_id="$(printf '%s' "$complete_response" | jq -r '.storage_object.id')"

log_step "create beta public share link"
share_body="$(jq -n '{max_downloads:10}')"
share_response="$(json_post "/workspaces/$workspace_id/objects/$object_id/share-links" "$share_body" "$session_token")"

log_step "verify legacy API key creation is disabled"
api_key_body="$(jq -n '{name:"beta-smoke",scopes:["files:read","quota:read","share_links:read"]}')"
api_key_status="$(curl --silent --show-error --location --max-time 20 \
  --output /tmp/nvbes-beta-api-key-create.json \
  --write-out '%{http_code}' \
  --header "content-type: application/json" \
  --header "authorization: Bearer $session_token" \
  --data "$api_key_body" \
  "$DRIVE_API_BASE_URL/workspaces/$workspace_id/api-keys")"
if [ "$api_key_status" != "410" ]; then
  cat /tmp/nvbes-beta-api-key-create.json >&2
  fail "expected legacy API key creation to return HTTP 410, got $api_key_status"
fi

jq -n \
  --arg workspaceId "$workspace_id" \
  --arg folderId "$folder_id" \
  --arg objectId "$object_id" \
  --arg shareUrl "$(printf '%s' "$share_response" | jq -r '.share_url')" \
  --arg apiKeyCreationStatus "$api_key_status" \
  '{
    workspaceId:$workspaceId,
    folderId:$folderId,
    objectId:$objectId,
    shareUrl:$shareUrl,
    apiKeyCreationStatus:$apiKeyCreationStatus
  }'
