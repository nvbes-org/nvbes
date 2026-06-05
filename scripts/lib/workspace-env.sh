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
}
