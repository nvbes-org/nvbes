#!/usr/bin/env bash
# ==============================================================================
# nvbes Pre-flight Runner
# ==============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

node "${REPO_ROOT}/tools/ci/preflight.mjs" "$@"
