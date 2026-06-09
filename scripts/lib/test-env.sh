#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

# Load workspace-level environment variables for local dev and tests.
# shellcheck disable=SC1091
source "$ROOT_DIR/scripts/lib/workspace-env.sh"
load_workspace_env

if [ "${CI:-}" = "true" ] || [ "${GITHUB_ACTIONS:-}" = "true" ]; then
  export NVBES_REDIS_PASSWORD=""
fi

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

normalize_url() {
  printf '%s' "${1%/}"
}

http_body() {
  local url="$1"
  curl --fail --silent --show-error --location --max-time 15 "$url"
}

http_status() {
  local url="$1"
  curl --silent --show-error --location --max-time 15 --output /dev/null --write-out '%{http_code}' "$url"
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

  headers="$(curl --silent --show-error --location --max-time 15 --head "$url")"
  printf '%s' "$headers" | grep -iF "$expected" >/dev/null || fail "expected headers from $url to contain: $expected"
}
