#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
source "$SCRIPT_DIR/lib/test-env.sh"
# shellcheck disable=SC1091
source "$SCRIPT_DIR/lib/dev-ports.sh"

cd "$ROOT_DIR"
free_dev_port "${VITE_IDENTITY_WEB_PORT:-3000}" "identity-web"
exec pnpm --dir apps/identity-web dev
