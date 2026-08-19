#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

nvbes_env_caller_script="${BASH_SOURCE[1]:-workspace}"
nvbes_env_caller_script="${nvbes_env_caller_script##*/}"
nvbes_env_caller_script="${nvbes_env_caller_script%.sh}"
nvbes_env_caller_script="${nvbes_env_caller_script#dev-}"
nvbes_node_process_title="${NVBES_NODE_PROCESS_TITLE:-nvbes:$nvbes_env_caller_script}"

# Load workspace-level environment variables for local dev and tests.
# shellcheck disable=SC1091
source "$ROOT_DIR/scripts/lib/workspace-env.sh"
load_workspace_env "$nvbes_node_process_title"
unset nvbes_env_caller_script nvbes_node_process_title

if [ "${CI:-}" = "true" ] || [ "${GITHUB_ACTIONS:-}" = "true" ]; then
  export NVBES_REDIS_PASSWORD=""
fi

if [ -z "${LIBCLANG_PATH:-}" ]; then
  for llvm_dir in /usr/lib/llvm-*/lib /usr/lib/*-linux-gnu /usr/lib/llvm-* /usr/local/opt/llvm/lib /opt/homebrew/opt/llvm/lib; do
    if [ -d "$llvm_dir" ] && ls "$llvm_dir"/libclang* >/dev/null 2>&1; then
      export LIBCLANG_PATH="$llvm_dir"
      break
    fi
  done
fi

export NVBES_ALLOW_DESTRUCTIVE_TEST_DATABASE="${NVBES_ALLOW_DESTRUCTIVE_TEST_DATABASE:-account-quality-v1}"

log_step() {
  printf '\n==> %s\n' "$1"
}

fail() {
  printf 'error: %s\n' "$1" >&2
  exit 1
}

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || fail "missing required command: $1"
}

require_env() {
  local name="$1"
  if [ -z "${!name:-}" ]; then
    fail "missing required environment variable: $name"
  fi
}

require_destructive_account_test_database() {
  require_env NVBES_ALLOW_DESTRUCTIVE_TEST_DATABASE
  node "$ROOT_DIR/scripts/lib/validate-destructive-test-database.mjs"
}

require_account_test_redis() {
  require_env NVBES_REDIS_URL
  node "$ROOT_DIR/scripts/lib/validate-test-redis-target.mjs"
}

validate_staging_account_targets() {
  require_cmd node
  require_env NVBES_STAGING_WEB_BASE_URL
  require_env NVBES_STAGING_API_BASE_URL
  require_env NVBES_STAGING_ALLOWED_WEB_ORIGINS
  require_env NVBES_STAGING_ALLOWED_API_ORIGINS
  require_env ACCOUNT_PRODUCTION_DENIED_ORIGINS

  NVBES_TARGET_ENV=staging \
    NVBES_WEB_BASE_URL="$NVBES_STAGING_WEB_BASE_URL" \
    ACCOUNT_WEB_ALLOWED_ORIGINS="$NVBES_STAGING_ALLOWED_WEB_ORIGINS" \
    ACCOUNT_PRODUCTION_DENIED_ORIGINS="$ACCOUNT_PRODUCTION_DENIED_ORIGINS" \
    node "$ROOT_DIR/tools/account-quality/validate-load-target.mjs" web
  NVBES_TARGET_ENV=staging \
    ACCOUNT_SERVICE_BASE_URL="$NVBES_STAGING_API_BASE_URL" \
    ACCOUNT_LOAD_ALLOWED_ORIGINS="$NVBES_STAGING_ALLOWED_API_ORIGINS" \
    ACCOUNT_PRODUCTION_DENIED_ORIGINS="$ACCOUNT_PRODUCTION_DENIED_ORIGINS" \
    node "$ROOT_DIR/tools/account-quality/validate-load-target.mjs" service
}

normalize_url() {
  printf '%s' "${1%/}"
}

http_body() {
  local url="$1"
  local -a redirect_args=(--location)
  if [ "${NVBES_SMOKE_FORBID_REDIRECTS:-}" = "1" ]; then
    redirect_args=()
  fi
  curl --fail --silent --show-error "${redirect_args[@]}" --max-time 15 "$url"
}

http_status() {
  local url="$1"
  local -a redirect_args=(--location)
  if [ "${NVBES_SMOKE_FORBID_REDIRECTS:-}" = "1" ]; then
    redirect_args=()
  fi
  curl --silent --show-error "${redirect_args[@]}" --max-time 15 --output /dev/null --write-out '%{http_code}' "$url"
}

assert_http_status() {
  local url="$1"
  local expected="$2"
  local status

  status="$(http_status "$url")"
  if [ "$status" != "$expected" ]; then
    fail "expected HTTP $expected for $url, got $status"
  fi
}

assert_http_not_5xx() {
  local url="$1"
  local status

  status="$(http_status "$url")"
  case "$status" in
    3*)
      if [ "${NVBES_SMOKE_FORBID_REDIRECTS:-}" = "1" ]; then
        fail "expected non-redirect for $url, got $status"
      fi
      ;;
    5*) fail "expected non-5xx for $url, got $status" ;;
  esac
}

assert_contains() {
  local value="$1"
  local expected="$2"

  printf '%s' "$value" | grep -F "$expected" >/dev/null || fail "expected response to contain: $expected"
}

assert_header_contains() {
  local url="$1"
  local expected="$2"
  local headers

  local -a redirect_args=(--location)
  if [ "${NVBES_SMOKE_FORBID_REDIRECTS:-}" = "1" ]; then
    redirect_args=()
  fi
  headers="$(curl --silent --show-error "${redirect_args[@]}" --max-time 15 --head "$url")"
  printf '%s' "$headers" | grep -iF "$expected" >/dev/null || fail "expected headers from $url to contain: $expected"
}
