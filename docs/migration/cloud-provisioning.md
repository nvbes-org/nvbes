# Cloud Provisioning Evidence

## Status

- status: passed
- checks: 21
- passed: 21
- failed: 0

## Rules

- Every evidence row must be generated from the cloud provisioning source contract.
- `passed` requires the configured Go, README, target-structure, or OSS export source to contain the expected pattern.
- Summary counters must match evidence rows.
- Targeted tests must name the Go provisioning and control-plane packages required by parity.
- Generation provenance must identify sources, write command and targeted tests.

## Evidence

| Check | Status | Path |
|---|---:|---|
| Cloud provisioning exposes a request model | passed | `libs/go/provisioning/provisioning.go` |
| Cloud provisioning exposes a plan model | passed | `libs/go/provisioning/provisioning.go` |
| Provisioning plans use deterministic idempotency keys | passed | `libs/go/provisioning/provisioning.go` |
| Provisioning plans require audit event names | passed | `libs/go/provisioning/provisioning.go` |
| Provisioning plan contains ordered cloud resource steps | passed | `libs/go/provisioning/provisioning.go` |
| Provisioning class maps to SLA | passed | `libs/go/provisioning/provisioning.go` |
| Cloud rollback event name is declared | passed | `libs/go/provisioning/provisioning.go` |
| Provisioning idempotency and audit behavior are tested | passed | `libs/go/provisioning/provisioning_test.go` |
| Provisioning validation behavior is tested | passed | `libs/go/provisioning/provisioning_test.go` |
| Cloud audit event names are tested | passed | `libs/go/provisioning/provisioning_test.go` |
| Provisioning package is provider-neutral by contract | passed | `libs/go/provisioning/README.md` |
| Cloud control-plane runtime has health surface | passed | `libs/go/control-plane/foundation.go` |
| Cloud control-plane health is tested | passed | `libs/go/control-plane/foundation_test.go` |
| Cloud Control API boundary is documented | passed | `apps/cloud-control-api/README.md` |
| Cloud Console boundary is documented | passed | `apps/cloud-console/README.md` |
| Cloud app boundary exists | passed | `apps/cloud/README.md` |
| Cloud deployment boundary exists | passed | `deploy/cloud/README.md` |
| Cloud docs boundary exists | passed | `docs/cloud/README.md` |
| Target structure includes Cloud app boundary | passed | `docs/migration/target-structure.generated.json` |
| Target structure includes Go provisioning | passed | `docs/migration/target-structure.generated.json` |
| OSS export manifest excludes Cloud-only paths | passed | `tools/oss-export/manifest.json` |

## Decision

Cloud provisioning repository evidence is covered. Production cutover still requires environment-specific apply, rollback, SLA and support evidence.

## Regeneration

```bash
pnpm check:migration-cloud-provisioning
tools/migration/cloud-provisioning.mjs --write
```
