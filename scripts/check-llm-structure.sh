#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

errors=0

deep_rs_files="$(
  find apps libs/rust -type f -name '*.rs' \
    | grep '/src/.*/' || true
)"

if [[ -n "$deep_rs_files" ]]; then
  echo "Rust files must stay flat under src/; found nested paths:"
  echo "$deep_rs_files"
  errors=1
fi

while IFS= read -r file; do
  [[ -n "$file" ]] || continue
  lines=$(wc -l < "$file" | tr -d ' ')
  if [[ "$lines" -gt 500 ]]; then
    echo "ERROR: $file has $lines lines (limit: 500)"
    errors=1
  elif [[ "$lines" -gt 300 ]]; then
    echo "WARN:  $file has $lines lines (consider extraction)"
  fi
done < <(find apps libs/rust -type f -name '*.rs' | sort)

exit "$errors"
