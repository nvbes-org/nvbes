#!/bin/bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

echo "Generating OpenAPI specs..."

cargo run -p nvbes-identity-api -- --export-openapi > apps/identity-api/openapi.json
echo "  ✓ identity-api/openapi.json"
cp apps/identity-api/openapi.json libs/ts/identity-sdk-core/openapi.json
echo "  ✓ identity-sdk-core/openapi.json"

cargo run -p nvbes-drive-api -- --export-openapi > apps/drive-api/openapi.json
echo "  ✓ drive-api/openapi.json"

pnpm exec vp fmt --write \
  apps/identity-api/openapi.json \
  apps/drive-api/openapi.json \
  libs/ts/identity-sdk-core/openapi.json >/dev/null

echo "Generating TypeScript types..."
pnpm --dir libs/ts/identity-sdk-core generate:ts
pnpm exec vp fmt --write libs/ts/identity-sdk-core/src/types.gen.ts >/dev/null

echo "Done."
