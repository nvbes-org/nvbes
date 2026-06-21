# Privacy Export and Delete Evidence

## Status

- status: passed
- checks: 29
- passed: 29
- failed: 0

## Rules

- Every evidence row must be generated from the privacy export/delete source contract.
- `passed` requires the configured file or OpenAPI document to contain the expected pattern.
- Summary counters must match evidence rows.
- Targeted tests must name account export and worker payload checks required by parity.
- Generation provenance must identify sources, write command and targeted tests.

## Evidence

| Check | Status | Path |
|---|---:|---|
| OpenAPI exposes data export request | passed | `apps/identity-api/openapi.json` |
| OpenAPI exposes prepared export download | passed | `apps/identity-api/openapi.json` |
| OpenAPI exposes account deletion request | passed | `apps/identity-api/openapi.json` |
| OpenAPI export includes data export handlers | passed | `apps/identity-api/src/identity.http.openapi.rs` |
| OpenAPI export includes account delete handler | passed | `apps/identity-api/src/identity.http.openapi.rs` |
| Data export request requires recent step-up | passed | `apps/identity-api/src/identity.domains.auth.data_export.rs` |
| Data export request is rate limited | passed | `apps/identity-api/src/identity.domains.auth.data_export.rs` |
| Data export request enqueues confirmation email | passed | `apps/identity-api/src/identity.domains.auth.data_export.rs` |
| Data export request enqueues worker job | passed | `apps/identity-api/src/identity.domains.auth.data_export.rs` |
| Data export worker job is idempotent per user | passed | `apps/identity-api/src/identity.email.jobs.rs` |
| Export download requires recent step-up | passed | `apps/identity-api/src/identity.domains.auth.routes.session_mgmt.export.rs` |
| Export download disables caching | passed | `apps/identity-api/src/identity.domains.auth.routes.session_mgmt.export.rs` |
| API tests prove account export cache key scope | passed | `apps/identity-api/src/identity.domains.auth.data_export.rs` |
| API tests prove prepared export TTL | passed | `apps/identity-api/src/identity.domains.auth.data_export.rs` |
| Worker dispatch handles data export jobs | passed | `apps/identity-worker/src/identity.worker.jobs.execute.rs` |
| Worker builds account export | passed | `apps/identity-worker/src/identity.worker.jobs.process_data_export.rs` |
| Worker stores prepared export before notification | passed | `apps/identity-worker/src/identity.worker.jobs.process_data_export.rs` |
| Worker notifies the requester when export is ready | passed | `apps/identity-worker/src/identity.worker.jobs.process_data_export.rs` |
| Worker smoke test accepts valid export payload | passed | `apps/identity-worker/src/identity.worker.jobs.process_data_export.rs` |
| Worker smoke test rejects missing user id | passed | `apps/identity-worker/src/identity.worker.jobs.process_data_export.rs` |
| Worker smoke test rejects invalid user id | passed | `apps/identity-worker/src/identity.worker.jobs.process_data_export.rs` |
| Account deletion requires AAL2 step-up | passed | `apps/identity-api/src/identity.domains.auth.account_deletion.rs` |
| Account deletion is rate limited | passed | `apps/identity-api/src/identity.domains.auth.account_deletion.rs` |
| Account deletion marks user deleted | passed | `apps/identity-api/src/identity.domains.auth.account_deletion.rs` |
| Account deletion marks principal deleted | passed | `apps/identity-api/src/identity.domains.auth.account_deletion.rs` |
| Account deletion revokes sessions transactionally | passed | `apps/identity-api/src/identity.domains.auth.account_deletion.rs` |
| Account deletion clears Redis sessions | passed | `apps/identity-api/src/identity.domains.auth.account_deletion.rs` |
| Account deletion publishes user suspension | passed | `apps/identity-api/src/identity.domains.auth.account_deletion.rs` |
| Account deletion publishes owned workspace deletion | passed | `apps/identity-api/src/identity.domains.auth.account_deletion.rs` |

## Decision

Audit/Privacy export and delete request evidence is covered for repository cutover gates.

## Regeneration

```bash
pnpm check:migration-privacy-export-delete
tools/migration/privacy-export-delete.mjs --write
```
