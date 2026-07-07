# Migration Gate Evidence

## Status

- No gate is approved for production cutover unless its generated row is `passed` or `accepted`, has decision `go`, and carries evidence.

- gates: 9
- pending: 5
- passed: 4
- accepted: 0
- failed: 0

## Rules

- Every blueprint gate must have an owner.
- Every gate must attach evidence before it can be marked `go`.
- Every `passed` or `accepted` gate evidence path must still exist in the repository.
- A production cutover requires all gates to be `passed` or `accepted`.
- A missing gate, missing evidence or `no-go` decision blocks cutover.
- Pending cutover gates must expose the live evidence packet and notes required to unlock them.
- Generation provenance must identify the blueprint source, write command and strict cutover command.

## Gates

| Gate | Owner | Status | Decision | Evidence | Notes |
|---|---|---|---|---|---|
| G0 Freeze | Migration lead | passed | go | `docs/migration/inventory.generated.json`<br>`docs/migration/data-map.generated.json`<br>`docs/migration/secret-map.generated.json`<br>`docs/migration/job-map.generated.json`<br>`docs/migration/resource-map.generated.json`<br>`docs/migration/owner-signoff-matrix.md`<br>`docs/migration/parity-matrix.md` | Repository freeze gate is satisfied by generated inventory and decision maps with zero pending source data, secret, job, or resource decisions. |
| G1 Fondation | Platform lead | passed | go | `docs/migration/target-structure.generated.json`<br>`docs/migration/codegen.generated.json`<br>`docs/migration/supply-chain.generated.json`<br>`docs/migration/runtime-foundation.generated.json`<br>`tools/boundary-checks/check-product-boundaries.mjs`<br>`scripts/check-nx-boundaries.mjs`<br>`scripts/check-rust-oss-boundaries.mjs`<br>`tools/oss-export/checks.mjs` | Automated foundation gate is satisfied by active monorepo workspaces, codegen, supply-chain, runtime and boundary checks. |
| G2 Primitives | Platform lead | passed | go | `docs/migration/platform-primitives.generated.json`<br>`libs/rust/audit/src/lib.rs`<br>`libs/rust/platform/src/platform.outbox.rs`<br>`libs/rust/ports/src/ports.outbox.rs`<br>`libs/rust/tenancy/src/lib.rs`<br>`libs/rust/core/src/http.error.rs`<br>`libs/rust/core/src/idempotency.rs`<br>`libs/rust/observability/src/lib.rs`<br>`contracts/events/manifest.json` | Primitive gate is satisfied by tested audit, outbox, tenancy, error, observability, idempotency and event-schema controls. |
| G3 Domaines | Migration lead | passed | go | `docs/migration/parity-matrix.md`<br>`docs/migration/domain-ledger.generated.json`<br>`docs/migration/domain-dod.generated.json`<br>`docs/migration/codegen.generated.json`<br>`contracts/openapi/manifest.json` | Domain gate is satisfied for repository controls by strict parity, generated public contracts and evidence-backed domain ledgers. |
| G4 Frontends | Product leads | pending | no-go | `docs/migration/cutover-evidence-packet.generated.json`<br>`docs/migration/live-evidence-instances.generated.json`<br>`docs/migration/frontend-experience.generated.json`<br>`docs/migration/smoke-test-manifest.md` | Pending frontend sign-off; requires accepted g4-frontend-signoff live evidence before gate approval. |
| G5 Infra | Infra lead | pending | no-go | `docs/migration/cutover-evidence-packet.generated.json`<br>`docs/migration/live-evidence-instances.generated.json`<br>`docs/migration/infra-deploy.generated.json`<br>`docs/migration/backup-restore-manifest.md`<br>`docs/migration/rollback-report.md` | Pending infra sign-off; requires accepted g5-infra-signoff live restore and rollback evidence before gate approval. |
| G6 Repetitions | Data lead | pending | no-go | `docs/migration/cutover-evidence-packet.generated.json`<br>`docs/migration/live-evidence-instances.generated.json`<br>`docs/migration/rehearsal-ledger.md`<br>`docs/migration/reconciliation.template.json`<br>`docs/migration/rejects.md` | Pending migration rehearsals; requires accepted p11-rehearsals live evidence before gate approval. |
| G7 Cutover | Migration lead | pending | no-go | `docs/migration/cutover-evidence-packet.generated.json`<br>`docs/migration/live-evidence-instances.generated.json`<br>`docs/migration/cutover-journal.md`<br>`docs/migration/cutover-checklist.md`<br>`docs/migration/observability-readiness.md`<br>`docs/migration/smoke-test-manifest.md` | Pending production cutover; requires accepted p12-cutover and final reconciliation live evidence before gate approval. |
| G8 Decommission | Infra lead | pending | no-go | `docs/migration/cutover-evidence-packet.generated.json`<br>`docs/migration/live-evidence-instances.generated.json`<br>`docs/migration/decommission-manifest.md`<br>`docs/migration/post-migration-audit.md` | Pending decommission; requires accepted p13-decommission live evidence before gate approval. |

## Evidence Types

| Type | Example |
|---|---|
| command output | `pnpm check`, `cargo check --workspace` |
| report | reconciliation, rollback, post-migration audit |
| checksum | data migration checksum |
| CI link | immutable CI run URL |
| sign-off | owner-approved decision record |

## Regeneration

```bash
node tools/migration/gate-evidence.mjs --write
pnpm check:migration-gate-evidence
```

Before a production cutover, run the strict gate:

```bash
node tools/migration/gate-evidence.mjs --strict
```
