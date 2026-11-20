#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib/test-env.sh"

require_cmd cargo
require_cmd node
require_env NVBES_WEB_BASE_URL
require_env NVBES_API_BASE_URL

if ! curl -sf "${NVBES_API_BASE_URL}/health" >/dev/null 2>&1 && ! curl -sf "${NVBES_API_BASE_URL}/.well-known/jwks.json" >/dev/null 2>&1; then
  printf 'notice: live services not running at %s; skipping openapi contract probe.\n' "$NVBES_API_BASE_URL"
  exit 0
fi

node "$SCRIPT_DIR/test-openapi-contract.mjs"
