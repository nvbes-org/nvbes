# Reconciliation Report

## Status

Template initialized. No production reconciliation has been executed.

- `total_metadata_rows`: 6
- `none_metadata_rows`: 5
- `metadata_no_go_decisions`: 1
- `total_domain_rows`: 6
- `pending_domain_rows`: 0
- `passed_domain_rows`: 0
- `accepted_domain_rows`: 0
- `blocking_domain_rows`: 6

## Rules

- Markdown `go` decisions require concrete environment, snapshot, target commit,
  start time and completion time metadata.
- Markdown `go` decisions require parseable start/completion timestamps with
  completion at or after start.
- Markdown `go` decisions require every summary domain to be `passed` or
  `accepted`.
- All summary domains must keep counts, checksums, orphan and reject evidence
  concrete when marked `passed` or `accepted`.
- A blocking domain keeps the report at `no-go`.
- An accepted domain requires explicit owner-reviewed reconciliation evidence in
  the generated JSON report and reject log.

## Run Metadata

| Field | Value |
|---|---|
| environment | none |
| snapshot_id | none |
| target_commit | none |
| started_at | none |
| completed_at | none |
| decision | no-go until populated |

## Summary

| Domain | Counts | Checksums | Orphans | Rejects | Status |
|---|---|---|---|---|---|
| Identity | source/target account, credential, session and MFA row counts with zero unexplained delta | source/target checksum bundle for identity tables and credential rebuild decisions | orphan user, session and MFA factor report | reject log with fixed/accepted/rejected/blocking identity rows | blocking |
| Workspace/Authz | source/target tenant, membership, role and policy row counts with zero unexplained delta | source/target checksum bundle for workspace and authorization tables | orphan tenant, membership and role-binding report | reject log with fixed/accepted/rejected/blocking workspace rows | blocking |
| Drive | source/target metadata, folder, object, share and quota row counts with zero unexplained delta | metadata checksum bundle plus sampled object checksum report | orphan file, object, share and quota report | reject log with fixed/accepted/rejected/blocking drive rows | blocking |
| Billing/Usage | source/target plan, entitlement, usage ledger, invoice and webhook row counts with zero unexplained delta | source/target checksum bundle for billing state and usage ledger windows | orphan entitlement, invoice and ledger-entry report | reject log with fixed/accepted/rejected/blocking billing rows | blocking |
| Audit/Privacy | source/target audit event, export, deletion and retention row counts with zero unexplained delta | append-only audit checksum bundle and privacy artifact checksum report | orphan audit event, export and deletion request report | reject log with fixed/accepted/rejected/blocking audit/privacy rows | blocking |
| Developer Platform | source/target app, token, OAuth client, webhook and SDK registry row counts with zero unexplained delta | source/target checksum bundle for developer apps, tokens and webhook deliveries | orphan app, token, OAuth client and webhook delivery report | reject log with fixed/accepted/rejected/blocking developer rows | blocking |

## Evidence

Attach a complete evidence bundle for each rehearsal or production run:

- generated reconciliation JSON report with schema validation result;
- export, transform and import command output logs with run IDs;
- source and target row-count reports for every summary domain;
- checksum bundle files for tables, object samples and audit chains;
- orphan reports for identity, workspace, drive, billing, audit/privacy and
  developer records;
- reject log snapshot with each reject marked blocking, fixed or owner accepted;
- owner review record for every accepted domain, accepted reject or non-zero
  delta;
- immutable source snapshot reference, target commit and image digest manifest;
- cutover journal link when the report is used for production cutover;
- live evidence instance reference when the report satisfies a packet
  requirement.

Machine-verifiable reports use `docs/migration/reconciliation-report.schema.json`.
The template is `docs/migration/reconciliation.template.json`.

Validate a rehearsal report with:

```bash
node tools/migration/reconcile.mjs --env staging --report docs/migration/reconciliation.<run>.json
```

Validate the template only with:

```bash
node tools/migration/reconcile.mjs --env template --report docs/migration/reconciliation.template.json --allow-template
```

## Decision

No cutover decision is approved from this template.
