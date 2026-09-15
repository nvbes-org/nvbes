#!/usr/bin/env bash

set -euo pipefail

workspace_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
echo "=== Running nvbes Chaos Engineering & Resilience Suite ==="

node "$workspace_root/tools/chaos/chaos-fault-harness.mjs"
node --test "$workspace_root/tools/chaos/chaos-fault-harness.test.mjs"

echo "=== All Chaos Engineering scenarios verified successfully ==="
