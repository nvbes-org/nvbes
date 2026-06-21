# Post-Migration Audit

## Status

Template initialized. Not applicable before cutover.

- `total_signoff_rows`: 5
- `pending_signoff_rows`: 5
- `approved_signoff_rows`: 0
- `accepted_signoff_rows`: 0
- `failed_signoff_rows`: 0
- `blocking_signoff_rows`: 0
- `total_audit_rows`: 7
- `pending_audit_rows`: 7
- `approved_audit_rows`: 0
- `accepted_audit_rows`: 0
- `passed_audit_rows`: 0
- `failed_audit_rows`: 0
- `blocking_audit_rows`: 0

## Rules

- An `approved` or `accepted` sign-off requires an explicit scope.
- An `approved` or `accepted` sign-off scope must reference concrete evidence
  such as a report, journal, inventory, log, artifact or check.
- An `approved` or `accepted` sign-off requires every audit item to be `approved`, `accepted`, or `passed`.
- An `approved`, `accepted` or `passed` audit item requires concrete evidence
  such as a report, journal, inventory, log, artifact, CI check, S3/GS/OCI
  reference, command result, link or ID.
- `pending` sign-offs or audit items block decommission approval.

## Required Sign-Off

| Owner | Scope | Status |
|---|---|---|
| Migration lead | docs/migration/cutover-journal.md final decision report, gate-evidence ledger, live evidence packet and postcutover gate evidence bundle | pending |
| Security lead | secret revocation log, access review report, audit export report, OSS export manifest and `pnpm check:oss-export` result | pending |
| Data lead | production reconciliation report, checksum bundle, reject disposition log, migration export retention record and backup/restore verification report | pending |
| Product leads | critical journey validation report, smoke-test manifest evidence, owner sign-off matrix and customer-facing hypercare summary | pending |
| Infra lead | docs/migration/decommission-manifest.md, rollback retention report, runtime inventory, disabled service/job logs and traffic cutover health report | pending |

## Audit Items

| Item | Evidence | Status |
|---|---|---|
| no legacy runtime service active | service inventory report, disabled runtime command log, endpoint probe result, traffic metric report and archive record | pending |
| no legacy job active | job inventory report, stopped job log, scheduler disable record, next-run absence check and queue drain record | pending |
| no unused legacy secret active | secret rotation log, revocation report, access review record, environment checksum and post-revocation probe result | pending |
| no private doc in OSS export | `pnpm check:oss-export` result, OSS export manifest report, private-doc scan log and Security approval record | pending |
| no provider boundary violation | `pnpm check:oss-boundaries` result, provider boundary report, dependency graph snapshot and zero-violation owner record | pending |
| backups and restore verified | restore report, checksum result, retention policy record, restore duration measurement and immutable backup reference | pending |
| hypercare complete | support report, owner handoff log, open incident list, customer communication record and residual-risk acceptance | pending |

## Decision

Decommission is not complete until every audit item is approved.
