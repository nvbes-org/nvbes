#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

cargo clippy \
  --package nvbes-account-service \
  --package nvbes-account-worker \
  --package nvbes-audit \
  --package nvbes-core \
  --package nvbes-dpop \
  --package nvbes-email \
  --package nvbes-identity-sdk \
  --package nvbes-observability \
  --package nvbes-product-account \
  --package nvbes-product-analytics \
  --package nvbes-redis \
  --package nvbes-region \
  --package nvbes-storage \
  --package nvbes-tenancy \
  --all-targets \
  --all-features \
  --locked \
  -- \
  -D warnings
