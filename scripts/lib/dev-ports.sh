#!/usr/bin/env bash

set -euo pipefail

dev_port_pids() {
  local port="$1"

  if command -v lsof >/dev/null 2>&1; then
    lsof -tiTCP:"$port" -sTCP:LISTEN 2>/dev/null || true
    return 0
  fi

  if command -v ss >/dev/null 2>&1; then
    ss -ltnp "sport = :$port" 2>/dev/null \
      | sed -nE 's/.*pid=([0-9]+).*/\1/p' \
      | sort -u
  fi
}

wait_for_port_exit() {
  local pid="$1"
  local attempts="${2:-25}"

  for _ in $(seq 1 "$attempts"); do
    if ! kill -0 "$pid" 2>/dev/null; then
      return 0
    fi
    sleep 0.2
  done

  return 1
}

dev_port_pid_is_protected() {
  local pid="$1"
  local command

  command="$(ps -p "$pid" -o command= 2>/dev/null || true)"
  case "$command" in
    *"Code Helper"* | *"Visual Studio Code"*)
      return 0
      ;;
    *)
      return 1
      ;;
  esac
}

free_dev_port() {
  local port="$1"
  local label="${2:-port $port}"
  local pids
  local kill_pids=""

  if [ "${NVBES_DEV_PORT_KILL:-1}" = "0" ]; then
    return 0
  fi

  pids="$(dev_port_pids "$port" | tr '\n' ' ')"
  if [ -z "${pids// }" ]; then
    return 0
  fi

  for pid in $pids; do
    if dev_port_pid_is_protected "$pid"; then
      printf 'Skipping protected process %s on port %s for %s\n' "$pid" "$port" "$label" >&2
    else
      kill_pids="${kill_pids}${pid} "
    fi
  done

  if [ -z "${kill_pids// }" ]; then
    return 0
  fi

  printf 'Freeing %s on port %s: %s\n' "$label" "$port" "$kill_pids"

  for pid in $kill_pids; do
    kill -TERM "$pid" 2>/dev/null || true
  done

  for pid in $kill_pids; do
    if ! wait_for_port_exit "$pid"; then
      printf 'Force killing %s process %s on port %s\n' "$label" "$pid" "$port" >&2
      kill -KILL "$pid" 2>/dev/null || true
    fi
  done
}

terminate_child_jobs() {
  local pids

  pids="$(jobs -pr | tr '\n' ' ')"
  if [ -z "${pids// }" ]; then
    return 0
  fi

  for pid in $pids; do
    kill -TERM "$pid" 2>/dev/null || true
  done

  for pid in $pids; do
    if ! wait_for_port_exit "$pid"; then
      kill -KILL "$pid" 2>/dev/null || true
    fi
  done
}
