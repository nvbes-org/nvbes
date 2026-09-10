#!/bin/bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

echo "Generating OpenAPI specs..."

cargo run --quiet -p nvbes-identity-service -- --export-openapi > apps/identity-service/openapi.json
echo "  ✓ identity-service/openapi.json"
cp apps/identity-service/openapi.json libs/ts/identity-sdk-core/openapi.json
echo "  ✓ identity-sdk-core/openapi.json"

cargo run --quiet -p nvbes-account-service -- --export-openapi > apps/account-service-next/openapi.json
echo "  ✓ account-service-next/openapi.json"

cargo run --quiet -p nvbes-developer-service -- --export-openapi > apps/developer-service/openapi.json
echo "  ✓ developer-service/openapi.json"

cargo run --quiet -p nvbes-cloud-service -- --export-openapi > apps/cloud-service/openapi.json
echo "  ✓ cloud-service/openapi.json"

cargo run --quiet -p nvbes-backoffice-service -- --export-openapi > apps/backoffice-service/openapi.json
echo "  ✓ backoffice-service/openapi.json"
cp apps/backoffice-service/openapi.json libs/ts/backoffice-service-sdk-core/openapi.json
echo "  ✓ backoffice-service-sdk-core/openapi.json"

node -e "const fs=require('fs'); const compact=['apps/identity-service/openapi.json','apps/account-service-next/openapi.json','apps/developer-service/openapi.json']; const pretty=['apps/cloud-service/openapi.json','apps/backoffice-service/openapi.json','libs/ts/identity-sdk-core/openapi.json','libs/ts/backoffice-service-sdk-core/openapi.json']; for (const f of compact) { const data=JSON.parse(fs.readFileSync(f,'utf8')); fs.writeFileSync(f, JSON.stringify(data)+'\n'); } for (const f of pretty) { const data=JSON.parse(fs.readFileSync(f,'utf8')); fs.writeFileSync(f, JSON.stringify(data,null,2)+'\n'); }"

echo "Generating TypeScript types..."
pnpm --dir libs/ts/identity-sdk-core generate:ts
pnpm --dir libs/ts/backoffice-service-sdk-core generate:ts

echo "Done."
