#!/usr/bin/env bash

set -euo pipefail

load_workspace_env() {
  local env_file="$ROOT_DIR/.env"

  export NVBES_WORKSPACE_ROOT="$ROOT_DIR"

  if [ ! -f "$env_file" ]; then
    return 0
  fi

  set -a
  # shellcheck disable=SC1090
  source "$env_file"
  set +a

  if [ "${NVBES_DEVCONTAINER:-}" = "true" ]; then
    export NVBES_DATABASE_URL="${NVBES_DEVCONTAINER_DATABASE_URL:-postgres://postgres:postgres@postgres:5432/nvbes}"
    export NVBES_DRIVE_DATABASE_URL="${NVBES_DEVCONTAINER_DRIVE_DATABASE_URL:-postgres://postgres:postgres@postgres:5432/nvbes_drive}"
    export NVBES_REDIS_URL="${NVBES_DEVCONTAINER_REDIS_URL:-redis://:redis_dev@redis:6379}"
    export STORAGE_PUBLIC_ENDPOINT="${NVBES_DEVCONTAINER_STORAGE_PUBLIC_ENDPOINT:-${STORAGE_PUBLIC_ENDPOINT:-${STORAGE_ENDPOINT:-http://localhost:18333}}}"
    export STORAGE_ENDPOINT="${NVBES_DEVCONTAINER_STORAGE_ENDPOINT:-http://seaweedfs:8333}"
  fi
}
