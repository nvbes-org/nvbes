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

cargo run -p nvbes-internal-admin -- --export-openapi > apps/internal-admin/openapi.json
echo "  ✓ internal-admin/openapi.json"
cp apps/internal-admin/openapi.json libs/ts/internal-admin-sdk-core/openapi.json
echo "  ✓ internal-admin-sdk-core/openapi.json"

node -e "const fs=require('fs'); const compact=['apps/identity-api/openapi.json']; const pretty=['apps/drive-api/openapi.json','apps/internal-admin/openapi.json','libs/ts/identity-sdk-core/openapi.json','libs/ts/internal-admin-sdk-core/openapi.json']; for (const f of compact) { const data=JSON.parse(fs.readFileSync(f,'utf8')); fs.writeFileSync(f, JSON.stringify(data)+'\n'); } for (const f of pretty) { const data=JSON.parse(fs.readFileSync(f,'utf8')); fs.writeFileSync(f, JSON.stringify(data,null,2)+'\n'); }"

echo "Generating TypeScript types..."
pnpm --dir libs/ts/identity-sdk-core generate:ts
pnpm --dir libs/ts/internal-admin-sdk-core generate:ts

echo "Done."
