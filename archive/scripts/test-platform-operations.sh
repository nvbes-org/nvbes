#!/usr/bin/env bash
set -euo pipefail

echo "=== [Step 1] Checking workspace compilation ==="
cargo check --workspace

echo "=== [Step 2] Running nvbes-platform unit and integration tests ==="
cargo nextest run --package nvbes-platform
bash scripts/test-platform-operations-database.sh

echo "=== [Step 3] Running container contract test ==="
node apps/platform-operations-service/tests/container-contract.test.mjs

echo "=== [Step 4] Running runtime proof ==="
./tools/deployment/prove-platform-operations-runtime.sh

echo "=== [Step 5] Checking line limits (< 300 lines per file) ==="
OVER_LIMIT=$(wc -l libs/rust/platform/src/*.rs docs/operations/runbook-*.md | awk '$1 > 300 && $2 != "total" {print $2, $1}')
if [[ -n "$OVER_LIMIT" ]]; then
  echo "Error: The following files exceed 300 lines:"
  echo "$OVER_LIMIT"
  exit 1
fi
echo "All domain files are strictly under 300 lines."

echo "=== All Platform Operations checks PASSED ==="
