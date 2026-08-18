# Privacy Export and Delete Evidence

## Status

- status: passed
- checks: 11
- passed: 11
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
| OpenAPI exposes data export request | passed | `apps/account-service-next/openapi.json` |
| OpenAPI exposes prepared export download | passed | `apps/account-service-next/openapi.json` |
| OpenAPI includes privacy export handlers | passed | `apps/account-service-next/src/account.openapi.rs` |
| Export request persists an Account-owned export | passed | `apps/account-service-next/src/account.privacy.routes.rs` |
| Export download is principal scoped | passed | `apps/account-service-next/src/account.privacy.routes.rs` |
| Expired exports are purged | passed | `apps/account-service-next/src/account.privacy.db.rs` |
| Multi-product participants are ordered | passed | `apps/account-service-next/src/account.privacy.db.rs` |
| Account export cache key is subject scoped | passed | `libs/rust/products/account/src/account.privacy.data_export.rs` |
| Prepared export TTL is bounded | passed | `libs/rust/products/account/src/account.privacy.data_export.rs` |
| Identity contributes privacy activity | passed | `apps/identity-service/src/identity.domains.auth.account_export.rs` |
| Account closure invalidates prepared exports | passed | `apps/account-service-next/src/account.closure.db.rs` |

## Decision

Audit/Privacy export and delete request evidence is covered for repository cutover gates.

## Regeneration

```bash
pnpm check:migration-privacy-export-delete
node tools/migration/privacy-export-delete.mjs --write
```
