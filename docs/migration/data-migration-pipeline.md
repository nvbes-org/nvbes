# Data Migration Pipeline Evidence

## Status

- status: passed
- domains: 7
- domains_passed: 7
- checks: 46
- passed: 46
- failed: 0

## Rules

- Every domain row must match the data migration domain contract.
- Every evidence row must be generated from the data migration source contract.
- Summary counters must match domain and evidence rows.
- Production cutover still requires executed reconciliation reports, not template evidence.
- Generation provenance must identify sources, write command and targeted tests.

## Domains

| Domain | Status | Source tables | Reconciliation |
|---|---:|---:|---|
| Identity | passed | 32 | checksum, orphan_check, row_count |
| Workspace | passed | 24 | checksum, orphan_check, row_count |
| Drive | passed | 18 | checksum, object_or_link_invariant, orphan_check, row_count |
| Billing | passed | 84 | checksum, ledger_balance, row_count |
| Audit | passed | 6 | checksum, orphan_check, row_count |
| Privacy | passed | 6 | checksum, orphan_check, row_count |
| Developer Platform | passed | 15 | checksum, row_count |

## Evidence

| Check | Status | Path |
|---|---:|---|
| Reject log includes class, owner, impact, evidence and decision columns | passed | `docs/migration/rejects.md` |
| Reconciliation schema requires domain reports | passed | `docs/migration/reconciliation-report.schema.json` |
| Reconciliation tool blocks unaccepted row deltas | passed | `tools/migration/reconcile.mjs` |
| Reconciliation tool blocks unaccepted checksum mismatches | passed | `tools/migration/reconcile.mjs` |
| Reconciliation template remains explicit no-go | passed | `docs/migration/reconciliation.template.json` |
| Snapshot manifest requires checksum evidence | passed | `docs/migration/snapshot-manifest.md` |
| Release freeze includes billing mutation freeze | passed | `docs/migration/release-freeze-manifest.md` |
| Drive upload/download evidence covers object storage invariants | passed | `docs/migration/drive-upload-download.generated.json` |
| Drive share/revoke evidence covers share-link invariants | passed | `docs/migration/drive-share-revoke.generated.json` |
| Billing entitlement evidence is present | passed | `docs/migration/billing-entitlements.generated.json` |
| Billing webhook idempotency evidence is present | passed | `docs/migration/billing-webhook-idempotency.generated.json` |
| Identity data migration export contract | passed | `docs/migration/data-map.generated.json` |
| Identity data migration transform contract | passed | `docs/migration/data-map.generated.json` |
| Identity data migration import contract | passed | `docs/migration/data-map.generated.json` |
| Identity data migration reject log contract | passed | `docs/migration/data-map.generated.json` |
| Identity data migration checksum contract | passed | `docs/migration/data-map.generated.json` |
| Workspace data migration export contract | passed | `docs/migration/data-map.generated.json` |
| Workspace data migration transform contract | passed | `docs/migration/data-map.generated.json` |
| Workspace data migration import contract | passed | `docs/migration/data-map.generated.json` |
| Workspace data migration reject log contract | passed | `docs/migration/data-map.generated.json` |
| Workspace data migration checksum contract | passed | `docs/migration/data-map.generated.json` |
| Drive data migration export contract | passed | `docs/migration/data-map.generated.json` |
| Drive data migration transform contract | passed | `docs/migration/data-map.generated.json` |
| Drive data migration import contract | passed | `docs/migration/data-map.generated.json` |
| Drive data migration reject log contract | passed | `docs/migration/data-map.generated.json` |
| Drive data migration checksum contract | passed | `docs/migration/data-map.generated.json` |
| Billing data migration export contract | passed | `docs/migration/data-map.generated.json` |
| Billing data migration transform contract | passed | `docs/migration/data-map.generated.json` |
| Billing data migration import contract | passed | `docs/migration/data-map.generated.json` |
| Billing data migration reject log contract | passed | `docs/migration/data-map.generated.json` |
| Billing data migration checksum contract | passed | `docs/migration/data-map.generated.json` |
| Audit data migration export contract | passed | `docs/migration/data-map.generated.json` |
| Audit data migration transform contract | passed | `docs/migration/data-map.generated.json` |
| Audit data migration import contract | passed | `docs/migration/data-map.generated.json` |
| Audit data migration reject log contract | passed | `docs/migration/data-map.generated.json` |
| Audit data migration checksum contract | passed | `docs/migration/data-map.generated.json` |
| Privacy data migration export contract | passed | `docs/migration/data-map.generated.json` |
| Privacy data migration transform contract | passed | `docs/migration/data-map.generated.json` |
| Privacy data migration import contract | passed | `docs/migration/data-map.generated.json` |
| Privacy data migration reject log contract | passed | `docs/migration/data-map.generated.json` |
| Privacy data migration checksum contract | passed | `docs/migration/data-map.generated.json` |
| Developer Platform data migration export contract | passed | `docs/migration/data-map.generated.json` |
| Developer Platform data migration transform contract | passed | `docs/migration/data-map.generated.json` |
| Developer Platform data migration import contract | passed | `docs/migration/data-map.generated.json` |
| Developer Platform data migration reject log contract | passed | `docs/migration/data-map.generated.json` |
| Developer Platform data migration checksum contract | passed | `docs/migration/data-map.generated.json` |

## Decision

Data migration pipeline repository evidence is covered for export, transform, import, reject logs and checksums. Production cutover still requires executed reconciliation reports with accepted counts and checksums.

## Regeneration

```bash
pnpm check:migration-data-migration-pipeline
tools/migration/data-migration-pipeline.mjs --write
```
