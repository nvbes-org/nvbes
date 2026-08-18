# Target Monorepo Structure Ledger

## Status

- entries: 56
- present: 51
- missing: 5

## Rules

- Path rows must match the blueprint target monorepo tree.
- Summary counters must match generated path rows.
- Generation provenance must identify source, write command and strict cutover command.

## Paths

| Path | Status | Proof |
|---|---:|---|
| `apps` | present | `apps` |
| `apps/api` | missing | `apps/api` |
| `apps/cloud` | missing | `apps/cloud` |
| `apps/internal` | missing | `apps/internal` |
| `apps/public` | missing | `apps/public` |
| `apps/workers` | missing | `apps/workers` |
| `contracts` | present | `contracts` |
| `contracts/events` | present | `contracts/events` |
| `contracts/openapi` | present | `contracts/openapi` |
| `contracts/protobuf` | present | `contracts/protobuf` |
| `deploy` | present | `deploy` |
| `deploy/cloud` | present | `deploy/cloud` |
| `deploy/internal` | present | `deploy/internal` |
| `deploy/oss` | present | `deploy/oss` |
| `docs` | present | `docs` |
| `docs/adr` | present | `docs/adr` |
| `docs/blueprint` | present | `docs/blueprint` |
| `docs/cloud` | present | `docs/cloud` |
| `docs/internal` | present | `docs/internal` |
| `docs/migration` | present | `docs/migration` |
| `docs/oss` | present | `docs/oss` |
| `libs` | present | `libs` |
| `libs/go` | present | `libs/go` |
| `libs/go/control-plane` | present | `libs/go/control-plane` |
| `libs/go/kubernetes-operators` | present | `libs/go/kubernetes-operators` |
| `libs/go/network` | present | `libs/go/network` |
| `libs/go/provisioning` | present | `libs/go/provisioning` |
| `libs/python` | present | `libs/python` |
| `libs/python/ai-services` | present | `libs/python/ai-services` |
| `libs/python/analytics-pipelines` | present | `libs/python/analytics-pipelines` |
| `libs/python/data-quality` | present | `libs/python/data-quality` |
| `libs/python/ocr` | present | `libs/python/ocr` |
| `libs/rust` | present | `libs/rust` |
| `libs/rust/adapters-cloud` | present | `libs/rust/adapters-cloud` |
| `libs/rust/adapters-oss` | present | `libs/rust/adapters-oss` |
| `libs/rust/cloud` | present | `libs/rust/cloud` |
| `libs/rust/platform` | present | `libs/rust/platform` |
| `libs/rust/ports` | present | `libs/rust/ports` |
| `libs/rust/products` | present | `libs/rust/products` |
| `libs/ts` | present | `libs/ts` |
| `libs/ts/cloud-ui` | present | `libs/ts/cloud-ui` |
| `libs/ts/design-system` | present | `libs/ts/design-system` |
| `libs/ts/edge-runtime` | present | `libs/ts/edge-runtime` |
| `libs/ts/http-client` | present | `libs/ts/http-client` |
| `libs/ts/sdk-core` | present | `libs/ts/sdk-core` |
| `libs/ts/sdk-drive` | present | `libs/ts/sdk-drive` |
| `libs/ts/sdk-identity` | present | `libs/ts/sdk-identity` |
| `libs/ts/web-runtime` | present | `libs/ts/web-runtime` |
| `libs/ts/web-ui` | present | `libs/ts/web-ui` |
| `tools` | present | `tools` |
| `tools/boundary-checks` | present | `tools/boundary-checks` |
| `tools/codegen` | present | `tools/codegen` |
| `tools/load-tests` | present | `tools/load-tests` |
| `tools/migration` | present | `tools/migration` |
| `tools/oss-export` | present | `tools/oss-export` |
| `tools/security` | present | `tools/security` |

## Regeneration

```bash
pnpm check:migration-target-structure
node tools/migration/target-structure.mjs --write
```
