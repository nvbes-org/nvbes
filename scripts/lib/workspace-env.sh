#!/usr/bin/env bash

set -euo pipefail

load_workspace_env() {
  local env_file="$ROOT_DIR/.env"
  local dotenv_exports
  local node_process_title="${1:-nvbes:workspace-env}"

  export NVBES_WORKSPACE_ROOT="$ROOT_DIR"

  if [[ ! -f "$env_file" ]]; then
    return 0
  fi

  # Parse dotenv as data: never source the developer file as shell code.
  # Existing process and CI variables remain authoritative.
  dotenv_exports="$(node --title="$node_process_title" "$ROOT_DIR/scripts/env.mjs" export)" || return 1
  eval "$dotenv_exports"

  if [[ "${NVBES_DEVCONTAINER:-}" = "true" || -f "/.dockerenv" ]]; then
    export NVBES_DATABASE_URL="${NVBES_DEVCONTAINER_DATABASE_URL:-postgres://postgres:postgres@postgres:5432/nvbes}"
    export NVBES_IDENTITY_DATABASE_URL="${NVBES_DEVCONTAINER_IDENTITY_DATABASE_URL:-postgres://postgres:postgres@postgres:5432/nvbes_identity}"
    export NVBES_IDENTITY_TEST_DATABASE_URL="${NVBES_DEVCONTAINER_IDENTITY_TEST_DATABASE_URL:-postgres://postgres:postgres@postgres:5432/nvbes_identity_test}"
    export NVBES_ACCOUNT_DATABASE_URL="${NVBES_DEVCONTAINER_ACCOUNT_DATABASE_URL:-postgres://postgres:postgres@postgres:5432/nvbes_account}"
    export NVBES_CLOUD_DATABASE_URL="${NVBES_DEVCONTAINER_CLOUD_DATABASE_URL:-postgres://postgres:postgres@postgres:5432/nvbes_cloud}"
    export NVBES_BILLING_DATABASE_URL="${NVBES_DEVCONTAINER_BILLING_DATABASE_URL:-postgres://postgres:postgres@postgres:5432/nvbes_billing}"
    export NVBES_DEVELOPER_DATABASE_URL="${NVBES_DEVCONTAINER_DEVELOPER_DATABASE_URL:-postgres://postgres:postgres@postgres:5432/nvbes_developer}"
    export NVBES_ENTERPRISE_DATABASE_URL="${NVBES_DEVCONTAINER_ENTERPRISE_DATABASE_URL:-postgres://postgres:postgres@postgres:5432/nvbes_enterprise}"
    export NVBES_EMAIL_DATABASE_URL="${NVBES_DEVCONTAINER_EMAIL_DATABASE_URL:-postgres://postgres:postgres@postgres:5432/nvbes_email}"
    export NVBES_SMTP_HOST="${NVBES_DEVCONTAINER_SMTP_HOST:-mailpit}"
    export NVBES_SMTP_PORT="${NVBES_DEVCONTAINER_SMTP_PORT:-1025}"
    export NVBES_SMTP_STARTTLS="${NVBES_DEVCONTAINER_SMTP_STARTTLS:-false}"
    export STORAGE_PUBLIC_ENDPOINT="${NVBES_DEVCONTAINER_STORAGE_PUBLIC_ENDPOINT:-${STORAGE_PUBLIC_ENDPOINT:-${STORAGE_ENDPOINT:-http://localhost:18333}}}"
    export STORAGE_ENDPOINT="${NVBES_DEVCONTAINER_STORAGE_ENDPOINT:-http://seaweedfs:8333}"
  fi

  export NVBES_ACCOUNT_AVATAR_STORAGE_MODE="${NVBES_ACCOUNT_AVATAR_STORAGE_MODE:-mock}"
  if [[ "$NVBES_ACCOUNT_AVATAR_STORAGE_MODE" = "s3" ]]; then
    export NVBES_ACCOUNT_AVATAR_STORAGE_BUCKET="${NVBES_ACCOUNT_AVATAR_STORAGE_BUCKET:-${STORAGE_BUCKET:-nvbes-drive}}"
    export NVBES_ACCOUNT_AVATAR_STORAGE_ENDPOINT="${NVBES_ACCOUNT_AVATAR_STORAGE_ENDPOINT:-${STORAGE_ENDPOINT:-http://localhost:8333}}"
    export NVBES_ACCOUNT_AVATAR_STORAGE_PUBLIC_ENDPOINT="${NVBES_ACCOUNT_AVATAR_STORAGE_PUBLIC_ENDPOINT:-${STORAGE_PUBLIC_ENDPOINT:-${NVBES_ACCOUNT_AVATAR_STORAGE_ENDPOINT}}}"
    export NVBES_ACCOUNT_AVATAR_STORAGE_REGION="${NVBES_ACCOUNT_AVATAR_STORAGE_REGION:-${STORAGE_REGION:-eu-west-1}}"
    export NVBES_ACCOUNT_AVATAR_STORAGE_ACCESS_KEY="${NVBES_ACCOUNT_AVATAR_STORAGE_ACCESS_KEY:-${STORAGE_ACCESS_KEY:-nvbes}}"
    export NVBES_ACCOUNT_AVATAR_STORAGE_SECRET_KEY="${NVBES_ACCOUNT_AVATAR_STORAGE_SECRET_KEY:-${STORAGE_SECRET_KEY:-nvbes-dev-secret}}"
  fi
}
