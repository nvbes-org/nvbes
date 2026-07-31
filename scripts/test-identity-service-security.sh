#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

# Security behavior is intentionally distributed across authentication, OAuth,
# authorization, middleware, database/RLS and contract modules. A substring
# filter silently omits those boundaries, so this gate executes the complete
# Identity service suite through its fail-closed infrastructure preflight.
exec bash "$ROOT_DIR/scripts/test-identity-service.sh"
