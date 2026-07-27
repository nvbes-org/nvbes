#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
source "$SCRIPT_DIR/lib/test-env.sh"

require_cmd docker

export NVBES_OTLP_ENDPOINT="${NVBES_OTLP_ENDPOINT:-http://127.0.0.1:14317}"
export NVBES_PROFILING_ENABLED="${NVBES_PROFILING_ENABLED:-true}"
export NVBES_PROFILING_ENDPOINT="${NVBES_PROFILING_ENDPOINT:-http://127.0.0.1:4040}"
export NVBES_REDIS_PASSWORD="${NVBES_REDIS_PASSWORD:-redis_dev}"
export NVBES_ACCOUNT_WORKER_METRICS_BIND_ADDR="${NVBES_ACCOUNT_WORKER_METRICS_BIND_ADDR:-127.0.0.1:4102}"
export NVBES_LOG_FORMAT="${NVBES_LOG_FORMAT:-json}"
export RUST_LOG="${RUST_LOG:-info}"
export NVBES_OBSERVABILITY_LOG_DIR="$ROOT_DIR/infrastructure/local/observability/app-logs"
export VITE_FARO_URL_ACCOUNT_WEB="${VITE_FARO_URL_ACCOUNT_WEB:-http://localhost:12347/collect}"
export VITE_FARO_ENVIRONMENT="${VITE_FARO_ENVIRONMENT:-development}"
export VITE_FARO_TRACING_ORIGINS="${VITE_FARO_TRACING_ORIGINS:-http://localhost:4000,http://localhost:3001}"
export VITE_FARO_SESSION_SAMPLE_RATE="${VITE_FARO_SESSION_SAMPLE_RATE:-1}"

docker compose \
  --profile infra \
  --profile observability \
  -f "$ROOT_DIR/infrastructure/local/docker-compose.yml" \
  up -d

exec bash "$SCRIPT_DIR/dev-account.sh"
