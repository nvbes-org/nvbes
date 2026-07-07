# Drive Upload Download Evidence

## Status

- status: passed
- checks: 48
- passed: 48
- failed: 0

## Rules

- Every evidence row must be generated from the drive upload/download source contract.
- `passed` requires the configured file or generated migration document to contain the expected pattern.
- Summary counters must match evidence rows.
- Targeted tests must name upload, download, range, and reconciliation checks required by parity.
- Generation provenance must identify sources, write command and targeted tests.

## Evidence

| Check | Status | Path |
|---|---:|---|
| Upload logic validates object names | passed | `apps/cloud-service/src/drive.domains.uploads.logic.rs` |
| Upload size cap is covered by unit test | passed | `apps/cloud-service/src/drive.domains.uploads.logic.rs` |
| Upload content encoding is covered by unit tests | passed | `apps/cloud-service/src/drive.domains.uploads.logic.rs` |
| Upload object keys are workspace/object/upload scoped | passed | `apps/cloud-service/src/drive.domains.uploads.logic.rs` |
| Upload logic signs object-storage upload URLs | passed | `apps/cloud-service/src/drive.domains.uploads.logic.rs` |
| Upload creation checks quota before accepting metadata | passed | `apps/cloud-service/src/drive.domains.uploads.core.rs` |
| Upload creation inserts pending storage object | passed | `apps/cloud-service/src/drive.domains.uploads.core.rs` |
| Upload creation inserts upload session | passed | `apps/cloud-service/src/drive.domains.uploads.core.rs` |
| Upload creation is audited | passed | `apps/cloud-service/src/drive.domains.uploads.core.rs` |
| TUS upload creates multipart object-storage upload | passed | `apps/cloud-service/src/drive.domains.uploads.core.rs` |
| Public API exposes upload creation | passed | `apps/cloud-service/src/drive.domains.public_api.routes.v1_handlers.uploads.rs` |
| Public API exposes upload completion | passed | `apps/cloud-service/src/drive.domains.public_api.routes.v1_handlers.uploads.rs` |
| Public upload API requires files:write scope | passed | `apps/cloud-service/src/drive.domains.public_api.routes.v1_handlers.uploads.rs` |
| Upload completion reads object-storage metadata | passed | `apps/cloud-service/src/drive.domains.uploads.lifecycle.complete.rs` |
| Upload completion downloads uploaded object for validation and scan | passed | `apps/cloud-service/src/drive.domains.uploads.lifecycle.complete.rs` |
| Upload completion verifies declared and stored size | passed | `apps/cloud-service/src/drive.domains.uploads.lifecycle.complete.rs` |
| Upload completion verifies SHA-256 checksum | passed | `apps/cloud-service/src/drive.domains.uploads.lifecycle.complete.rs` |
| Upload completion scans uploaded bytes | passed | `apps/cloud-service/src/drive.domains.uploads.lifecycle.complete.rs` |
| Upload completion activates or quarantines storage object | passed | `apps/cloud-service/src/drive.domains.uploads.lifecycle.complete.rs` |
| Upload completion records quota usage | passed | `apps/cloud-service/src/drive.domains.uploads.lifecycle.complete.rs` |
| Upload completion is audited as file uploaded or quarantined | passed | `apps/cloud-service/src/drive.domains.uploads.lifecycle.complete.rs` |
| TUS append uploads object-storage parts | passed | `apps/cloud-service/src/drive.domains.uploads.lifecycle.tus.append.rs` |
| TUS append completes multipart upload | passed | `apps/cloud-service/src/drive.domains.uploads.lifecycle.tus.append.rs` |
| TUS finalization records quota usage | passed | `apps/cloud-service/src/drive.domains.uploads.lifecycle.tus.finalize.rs` |
| Drive download stream route is exposed | passed | `apps/cloud-service/src/drive.domains.files.routes.download.rs` |
| Drive download URL route is exposed | passed | `apps/cloud-service/src/drive.domains.files.routes.download.rs` |
| Download routes require DownloadFile authorization | passed | `apps/cloud-service/src/drive.domains.files.routes.download.rs` |
| Download stream reads full object from object storage | passed | `apps/cloud-service/src/drive.domains.files.transfer.stream.rs` |
| Download stream supports object-storage range reads | passed | `apps/cloud-service/src/drive.domains.files.transfer.stream.rs` |
| Download stream records bandwidth usage | passed | `apps/cloud-service/src/drive.domains.files.transfer.stream.rs` |
| Download stream is audited | passed | `apps/cloud-service/src/drive.domains.files.transfer.stream.rs` |
| Download URL flow signs object-storage download URLs | passed | `apps/cloud-service/src/drive.domains.files.transfer.download_url.rs` |
| Download URL flow records bandwidth usage | passed | `apps/cloud-service/src/drive.domains.files.transfer.download_url.rs` |
| Public API exposes download URL creation | passed | `apps/cloud-service/src/drive.domains.public_api.routes.v1_handlers.files.download.rs` |
| Public download API requires files:read scope | passed | `apps/cloud-service/src/drive.domains.public_api.routes.v1_handlers.files.download.rs` |
| Downloadability rules are covered by tests | passed | `apps/cloud-service/src/drive.domains.files.transfer.range.rs` |
| Download range behavior is covered by tests | passed | `apps/cloud-service/src/drive.domains.files.transfer.range.rs` |
| Invalid ranges are covered by tests | passed | `apps/cloud-service/src/drive.domains.files.transfer.range.rs` |
| Worker purges deleted object-storage keys and metadata | passed | `apps/cloud-worker/src/drive.workers.maintenance.storage.rs` |
| Worker purges quarantined object-storage keys and metadata | passed | `apps/cloud-worker/src/drive.workers.maintenance.storage.rs` |
| Worker deletes object-storage keys before metadata cleanup | passed | `apps/cloud-worker/src/drive.workers.maintenance.storage.rs` |
| Worker dispatch includes storage purge jobs | passed | `apps/cloud-worker/src/drive.workers.executor.dispatch.rs` |
| Drive OpenAPI source includes public upload/download routes | passed | `apps/cloud-service/src/drive.http.openapi.rs` |
| Generated OpenAPI includes public upload path | passed | `apps/cloud-service/openapi.json` |
| Generated OpenAPI includes public download URL path | passed | `apps/cloud-service/openapi.json` |
| Storage objects require object/link reconciliation | passed | `docs/migration/data-map.md` |
| Upload sessions are explicitly mapped for migration | passed | `docs/migration/data-map.md` |
| Storage jobs require storage reconciliation evidence | passed | `docs/migration/job-map.generated.json` |

## Decision

Drive upload/download parity evidence is covered for repository cutover gates. Production cutover still requires accepted object-storage reconciliation counts.

## Regeneration

```bash
pnpm check:migration-drive-upload-download
node tools/migration/drive-upload-download.mjs --write
```
