# Audit Append-Only Evidence

## Status

- status: passed
- checks: 34
- passed: 34
- failed: 0

## Rules

- Every evidence row must be generated from the audit append-only source contract.
- `passed` requires the configured migration or shared audit crate file to contain the expected pattern.
- Summary counters must match evidence rows.
- The generated source list must include both product migrations and the shared audit crate.
- Generation provenance must identify sources and write command.

## Evidence

| Product | Check | Status | Path |
|---|---|---:|---|
| identity | pgcrypto digest extension | passed | `apps/identity-api/migrations/0001_initial_schema.sql` |
| identity | hash chain column | passed | `apps/identity-api/migrations/0001_initial_schema.sql` |
| identity | event hash column | passed | `apps/identity-api/migrations/0001_initial_schema.sql` |
| identity | hash chain function | passed | `apps/identity-api/migrations/0001_initial_schema.sql` |
| identity | hash partition | passed | `apps/identity-api/migrations/0001_initial_schema.sql` |
| identity | sha256 digest | passed | `apps/identity-api/migrations/0001_initial_schema.sql` |
| identity | previous hash included | passed | `apps/identity-api/migrations/0001_initial_schema.sql` |
| identity | insert hash trigger | passed | `apps/identity-api/migrations/0001_initial_schema.sql` |
| identity | insert trigger timing | passed | `apps/identity-api/migrations/0001_initial_schema.sql` |
| identity | mutation blocker function | passed | `apps/identity-api/migrations/0001_initial_schema.sql` |
| identity | update blocker trigger | passed | `apps/identity-api/migrations/0001_initial_schema.sql` |
| identity | update trigger timing | passed | `apps/identity-api/migrations/0001_initial_schema.sql` |
| identity | delete blocker trigger | passed | `apps/identity-api/migrations/0001_initial_schema.sql` |
| identity | delete trigger timing | passed | `apps/identity-api/migrations/0001_initial_schema.sql` |
| identity | unique event hash index | passed | `apps/identity-api/migrations/0001_initial_schema.sql` |
| drive | pgcrypto digest extension | passed | `apps/drive-api/migrations/0001_initial_schema.sql` |
| drive | hash chain column | passed | `apps/drive-api/migrations/0001_initial_schema.sql` |
| drive | event hash column | passed | `apps/drive-api/migrations/0001_initial_schema.sql` |
| drive | hash chain function | passed | `apps/drive-api/migrations/0001_initial_schema.sql` |
| drive | hash partition | passed | `apps/drive-api/migrations/0001_initial_schema.sql` |
| drive | sha256 digest | passed | `apps/drive-api/migrations/0001_initial_schema.sql` |
| drive | previous hash included | passed | `apps/drive-api/migrations/0001_initial_schema.sql` |
| drive | insert hash trigger | passed | `apps/drive-api/migrations/0001_initial_schema.sql` |
| drive | insert trigger timing | passed | `apps/drive-api/migrations/0001_initial_schema.sql` |
| drive | mutation blocker function | passed | `apps/drive-api/migrations/0001_initial_schema.sql` |
| drive | update blocker trigger | passed | `apps/drive-api/migrations/0001_initial_schema.sql` |
| drive | update trigger timing | passed | `apps/drive-api/migrations/0001_initial_schema.sql` |
| drive | delete blocker trigger | passed | `apps/drive-api/migrations/0001_initial_schema.sql` |
| drive | delete trigger timing | passed | `apps/drive-api/migrations/0001_initial_schema.sql` |
| drive | unique event hash index | passed | `apps/drive-api/migrations/0001_initial_schema.sql` |
| shared-audit-crate | pool insert API | passed | `libs/rust/audit/src/lib.rs` |
| shared-audit-crate | transaction insert API | passed | `libs/rust/audit/src/lib.rs` |
| shared-audit-crate | event hash delegated to trigger | passed | `libs/rust/audit/src/lib.rs` |
| shared-audit-crate | event hash omitted from insert columns | passed | `libs/rust/audit/src/lib.rs` |

## Decision

Audit append-only parity evidence is covered for repository cutover gates.

## Regeneration

```bash
pnpm check:migration-audit-append-only
tools/migration/audit-append-only.mjs --write
```
