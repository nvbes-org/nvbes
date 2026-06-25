#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
START_PAGE="$ROOT_DIR/docs/development/start-with-nvbes-dev-container.md"
MARKER_DIR="${XDG_CACHE_HOME:-$HOME/.cache}/nvbes"
MARKER_FILE="$MARKER_DIR/start-page-opened"

if [ -f "$MARKER_FILE" ]; then
  exit 0
fi

mkdir -p "$MARKER_DIR"

if command -v code >/dev/null 2>&1; then
  code -r "$START_PAGE"
  touch "$MARKER_FILE"
else
  printf 'VS Code CLI is not available; start page skipped: %s\n' "$START_PAGE" >&2
fi
