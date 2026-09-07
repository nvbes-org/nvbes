#!/usr/bin/env bash
set -euo pipefail
: "${NVBES_PLATFORM_OPERATIONS_DATABASE_URL:?Use a dedicated disposable proof database}"
cargo build --locked --package nvbes-platform --bin nvbes-platform-operations
python3 tools/deployment/platform-operations-runtime-proof.py
