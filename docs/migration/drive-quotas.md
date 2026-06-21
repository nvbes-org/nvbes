# Drive Quotas Evidence

## Status

- status: passed
- checks: 37
- passed: 37
- failed: 0

## Rules

- Every evidence row must be generated from the drive quotas source contract.
- `passed` requires the configured file or OpenAPI document to contain the expected pattern.
- Summary counters must match evidence rows.
- Targeted tests must name quota and reconciliation checks required by parity.
- Generation provenance must identify sources, write command and targeted tests.

## Evidence

| Check | Status | Path |
|---|---:|---|
| Quota response exposes storage, bandwidth, block state, and alerts | passed | `apps/drive-api/src/drive.domains.quotas.types.rs` |
| Drive quota route is exposed | passed | `apps/drive-api/src/drive.domains.quotas.routes.rs` |
| Drive quota route requires ViewQuota authorization | passed | `apps/drive-api/src/drive.domains.quotas.routes.rs` |
| Public API quota route is exposed | passed | `apps/drive-api/src/drive.domains.public_api.routes.v1_handlers.quotas_audit.rs` |
| Public API quota route requires quota:read scope | passed | `apps/drive-api/src/drive.domains.public_api.routes.v1_handlers.quotas_audit.rs` |
| Quota service ensures quota row before reading | passed | `apps/drive-api/src/drive.domains.quotas.service.rs` |
| Quota service returns storage usage percent | passed | `apps/drive-api/src/drive.domains.quotas.service.rs` |
| Quota service returns upload block state | passed | `apps/drive-api/src/drive.domains.quotas.service.rs` |
| Quota service returns threshold alerts | passed | `apps/drive-api/src/drive.domains.quotas.service.rs` |
| Quota service blocks uploads that exceed storage limits | passed | `apps/drive-api/src/drive.domains.quotas.service.rs` |
| Quota service blocks uploads when already over critical threshold | passed | `apps/drive-api/src/drive.domains.quotas.service.rs` |
| Completed uploads record storage usage events | passed | `apps/drive-api/src/drive.domains.quotas.service.rs` |
| Completed uploads record file count usage events | passed | `apps/drive-api/src/drive.domains.quotas.service.rs` |
| Deleted files release storage through negative usage events | passed | `apps/drive-api/src/drive.domains.quotas.service.rs` |
| Downloads and public access record bandwidth usage | passed | `apps/drive-api/src/drive.domains.quotas.service.rs` |
| Quota DB locks usage rows before mutation | passed | `apps/drive-api/src/drive.domains.quotas.db.rs` |
| Quota usage events are idempotent by workspace and key | passed | `apps/drive-api/src/drive.domains.quotas.db.rs` |
| Quota DB recalculates current-month bandwidth cache | passed | `apps/drive-api/src/drive.domains.quotas.db.rs` |
| Upload creation checks quota before accepting upload | passed | `apps/drive-api/src/drive.domains.uploads.core.rs` |
| Upload completion rechecks quota against actual size | passed | `apps/drive-api/src/drive.domains.uploads.lifecycle.complete.rs` |
| Upload completion records quota usage | passed | `apps/drive-api/src/drive.domains.uploads.lifecycle.complete.rs` |
| TUS append checks quota against actual size | passed | `apps/drive-api/src/drive.domains.uploads.lifecycle.tus.append.rs` |
| File streaming records bandwidth usage | passed | `apps/drive-api/src/drive.domains.files.transfer.stream.rs` |
| Download URL flow records bandwidth usage | passed | `apps/drive-api/src/drive.domains.files.transfer.download_url.rs` |
| Public share access records bandwidth usage | passed | `apps/drive-api/src/drive.domains.share_links.public.rs` |
| Quota threshold crossings create audit events | passed | `apps/drive-api/src/drive.domains.quotas.observability.rs` |
| Quota warning/critical alert logic is covered by tests | passed | `apps/drive-api/src/drive.domains.quotas.logic.rs` |
| Quota upload block behavior is covered by tests | passed | `apps/drive-api/src/drive.domains.quotas.logic.rs` |
| Quota threshold crossing behavior is covered by tests | passed | `apps/drive-api/src/drive.domains.quotas.logic.rs` |
| Quota recalculation job constant exists | passed | `apps/drive-worker/src/drive.workers.maintenance.rs` |
| Quota worker recalculates used storage from active files | passed | `apps/drive-worker/src/drive.workers.maintenance.rs` |
| Quota worker tests the canonical used_storage_bytes column | passed | `apps/drive-worker/src/drive.workers.maintenance.rs` |
| Worker dispatch executes quota recalculation job | passed | `apps/drive-worker/src/drive.workers.executor.dispatch.rs` |
| Drive OpenAPI source includes quota public API route | passed | `apps/drive-api/src/drive.http.openapi.rs` |
| Generated OpenAPI includes quota path | passed | `apps/drive-api/openapi.json` |
| Quota usage has ledger-balance reconciliation decision | passed | `docs/migration/data-map.md` |
| Quota recalculation job is mapped | passed | `docs/migration/job-map.md` |

## Decision

Drive quota parity evidence is covered for repository cutover gates. Production cutover still requires accepted quota reconciliation counts.

## Regeneration

```bash
pnpm check:migration-drive-quotas
tools/migration/drive-quotas.mjs --write
```
