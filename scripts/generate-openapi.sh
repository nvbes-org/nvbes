#!/bin/bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

echo "Generating OpenAPI specs..."

cargo run -p nvbes-account-service -- --export-openapi > apps/account-service/openapi.json
echo "  ✓ account-service/openapi.json"
cp apps/account-service/openapi.json libs/ts/identity-sdk-core/openapi.json
echo "  ✓ identity-sdk-core/openapi.json"

cargo run -p nvbes-cloud-service -- --export-openapi > apps/cloud-service/openapi.json
echo "  ✓ cloud-service/openapi.json"

cargo run -p nvbes-backoffice-service -- --export-openapi > apps/backoffice-service/openapi.json
echo "  ✓ backoffice-service/openapi.json"
cp apps/backoffice-service/openapi.json libs/ts/backoffice-service-sdk-core/openapi.json
echo "  ✓ backoffice-service-sdk-core/openapi.json"

node -e "const fs=require('fs'); const compact=['apps/account-service/openapi.json']; const pretty=['apps/cloud-service/openapi.json','apps/backoffice-service/openapi.json','libs/ts/identity-sdk-core/openapi.json','libs/ts/backoffice-service-sdk-core/openapi.json']; for (const f of compact) { const data=JSON.parse(fs.readFileSync(f,'utf8')); fs.writeFileSync(f, JSON.stringify(data)+'\n'); } for (const f of pretty) { const data=JSON.parse(fs.readFileSync(f,'utf8')); fs.writeFileSync(f, JSON.stringify(data,null,2)+'\n'); }"

echo "Generating TypeScript types..."
pnpm --dir libs/ts/identity-sdk-core generate:ts
pnpm --dir libs/ts/backoffice-service-sdk-core generate:ts

echo "Done."
