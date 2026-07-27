#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
source "$SCRIPT_DIR/lib/test-env.sh"

faro_url="${VITE_FARO_URL_ACCOUNT_WEB:-}"
if [ -z "$faro_url" ]; then
  require_cmd gh
  faro_url="$(
    gh variable get GRAFANA_FARO_URL_ACCOUNT_WEB \
      --repo "${NVBES_GITHUB_REPOSITORY:-nvbes-org/nvbes}"
  )"
fi

if [ -z "$faro_url" ]; then
  echo "Grafana Cloud Faro URL is not configured." >&2
  exit 1
fi

export VITE_FARO_URL_ACCOUNT_WEB="$faro_url"
export VITE_FARO_ENVIRONMENT="${VITE_FARO_ENVIRONMENT:-development}"
export VITE_FARO_TRACING_ORIGINS="${VITE_FARO_TRACING_ORIGINS:-http://localhost:4000,http://localhost:3001}"
export VITE_FARO_SESSION_SAMPLE_RATE="${VITE_FARO_SESSION_SAMPLE_RATE:-1}"
export NVBES_REQUIRE_FARO_ACCOUNT_WEB=true

exec bash "$SCRIPT_DIR/dev-account.sh"
