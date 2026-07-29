#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

export CI=true

pnpm exec nx run account-quality:test
pnpm exec nx run account-quality:lint-rust
pnpm exec nx run-many -t test \
  --projects=account-web,http-client,identity-client,identity-sdk,identity-sdk-core,identity-sdk-web,web-runtime,web-ui \
  --parallel=3
pnpm exec nx run account-service:test-contract
pnpm exec nx run account-service:test-migrations
pnpm exec nx run account-service:test-security
pnpm exec nx run account-worker:test
pnpm exec nx run account-worker:test-container-contract
pnpm exec nx run http-client:test:coverage
