#!/usr/bin/env bash
# Portable exclusive lock for coverage/condition serialization.
# Prefers flock(1) (Linux/CI). Falls back to mkdir lock (macOS).
#
# Usage:
#   source scripts/lib/workspace-lock.sh
#   workspace_lock_acquire "$ROOT_DIR/target/coverage.lock" 3600
set -euo pipefail

workspace_lock_acquire() {
  local lockfile="$1"
  local timeout_sec="${2:-3600}"
  mkdir -p "$(dirname "$lockfile")"

  if command -v flock >/dev/null 2>&1; then
    exec 9>"$lockfile"
    if ! flock -w "$timeout_sec" 9; then
      printf 'error: timed out waiting for %s\n' "$lockfile" >&2
      return 1
    fi
    return 0
  fi

  local lockdir="${lockfile}.d"
  local start=$SECONDS
  while ! mkdir "$lockdir" 2>/dev/null; do
    if (( SECONDS - start > timeout_sec )); then
      printf 'error: timed out waiting for %s\n' "$lockfile" >&2
      return 1
    fi
    sleep 0.2
  done
  # shellcheck disable=SC2064
  trap "rmdir '$lockdir' 2>/dev/null || true" EXIT
  # Touch the flock-compatible path so operators can see the lock.
  : >"$lockfile"
}
