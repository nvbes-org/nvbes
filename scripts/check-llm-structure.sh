#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

errors=0
warn_count=0
err_count=0
total_checked=0

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
  total_checked=$((total_checked + 1))
  lines=$(wc -l < "$file" | tr -d ' ')
  if [[ "$lines" -gt 500 ]]; then
    echo "ERROR: $file has $lines lines (limit: 500 - AGENTS.md)"
    errors=1
    err_count=$((err_count + 1))
  elif [[ "$lines" -gt 300 ]]; then
    echo "WARN:  $file has $lines lines (consider extraction > 300 - AGENTS.md)"
    warn_count=$((warn_count + 1))
  fi
done < <(find apps libs/rust libs/ts -type f \( -name '*.rs' -o -name '*.ts' -o -name '*.tsx' -o -name '*.mjs' -o -name '*.js' \) \
  -not -path '*/.*' \
  -not -path '*/archive/*' \
  -not -path '*/node_modules/*' \
  -not -path '*/target/*' \
  -not -path '*/__snapshots__/*' \
  -not -name '*.gen.ts' \
  -not -name '*.d.ts' | sort)

echo "---"
echo "Structure & size limits check: $total_checked files checked ($warn_count warning(s), $err_count error(s))."

exit "$errors"
