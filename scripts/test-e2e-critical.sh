#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib/test-env.sh"

require_cmd cargo
require_cmd pnpm
require_env NVBES_WEB_BASE_URL
require_env NVBES_API_BASE_URL
require_env NVBES_DATABASE_URL

log_step "critical browser journeys"
pnpm --dir apps/identity-web test:e2e:critical
