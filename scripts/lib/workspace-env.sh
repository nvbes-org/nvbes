#!/usr/bin/env bash

set -euo pipefail

load_workspace_env() {
  local env_file="$ROOT_DIR/.env"
  local inherited_environment

  export NVBES_WORKSPACE_ROOT="$ROOT_DIR"

  if [ ! -f "$env_file" ]; then
    return 0
  fi

  # Explicit process/CI variables are authoritative. Snapshot exported values
  # before sourcing the developer file, then restore them while retaining only
  # defaults that were previously absent.
  inherited_environment="$(export -p)"
  set -a
  # shellcheck disable=SC1090
  source "$env_file"
  set +a
  # `declare` inside a function would create short-lived locals, so replay the
  # snapshot as exports to restore the caller's environment after this returns.
  eval "${inherited_environment//declare -x /export }"

  if [ "${NVBES_DEVCONTAINER:-}" = "true" ] || [ -f "/.dockerenv" ]; then
    export NVBES_DATABASE_URL="${NVBES_DEVCONTAINER_DATABASE_URL:-postgres://postgres:postgres@postgres:5432/nvbes}"
    export NVBES_IDENTITY_DATABASE_URL="${NVBES_DEVCONTAINER_IDENTITY_DATABASE_URL:-postgres://postgres:postgres@postgres:5432/nvbes_identity}"
    export NVBES_ACCOUNT_DATABASE_URL="${NVBES_DEVCONTAINER_ACCOUNT_DATABASE_URL:-postgres://postgres:postgres@postgres:5432/nvbes_account}"
    export NVBES_CLOUD_DATABASE_URL="${NVBES_DEVCONTAINER_CLOUD_DATABASE_URL:-postgres://postgres:postgres@postgres:5432/nvbes_cloud}"
    export NVBES_BILLING_DATABASE_URL="${NVBES_DEVCONTAINER_BILLING_DATABASE_URL:-postgres://postgres:postgres@postgres:5432/nvbes_billing}"
    export NVBES_DEVELOPER_DATABASE_URL="${NVBES_DEVCONTAINER_DEVELOPER_DATABASE_URL:-postgres://postgres:postgres@postgres:5432/nvbes_developer}"
    export NVBES_ENTERPRISE_DATABASE_URL="${NVBES_DEVCONTAINER_ENTERPRISE_DATABASE_URL:-postgres://postgres:postgres@postgres:5432/nvbes_enterprise}"
    export NVBES_DRIVE_DATABASE_URL="${NVBES_DEVCONTAINER_DRIVE_DATABASE_URL:-postgres://postgres:postgres@postgres:5432/nvbes_drive}"
    export NVBES_REDIS_URL="${NVBES_DEVCONTAINER_REDIS_URL:-redis://:redis_dev@redis:6379}"
    export STORAGE_PUBLIC_ENDPOINT="${NVBES_DEVCONTAINER_STORAGE_PUBLIC_ENDPOINT:-${STORAGE_PUBLIC_ENDPOINT:-${STORAGE_ENDPOINT:-http://localhost:18333}}}"
    export STORAGE_ENDPOINT="${NVBES_DEVCONTAINER_STORAGE_ENDPOINT:-http://seaweedfs:8333}"
  fi
}
