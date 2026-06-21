# Cutover Checklist

## Status

Initialized. Not approved for production.

- `total_checklist_rows`: 18
- `pending_rows`: 18
- `passed_rows`: 0
- `accepted_rows`: 0
- `failed_rows`: 0
- `blocking_rows`: 0

## Rules

- Checklist row status must be `pending`, `passed`, `accepted`, `failed` or
  `blocking`.
- A `passed` or `accepted` row requires concrete owner and evidence such as a
  SHA, digest, frozen list, report, link, ID, snapshot, export, import,
  reconciliation, edge change, health result, signed decision or channel record.
- The `T+180m` final decision row can be `passed` or `accepted` only when every
  prior checklist row is `passed` or `accepted`.
- `pending`, `failed` or `blocking` rows block production cutover.

## Pre-Cutover

| Item | Owner | Evidence | Status |
|---|---|---|---|
| commit target frozen | migration lead | signed target commit SHA, annotated release tag record, CI run URL and release-freeze manifest link | pending |
| images OCI immutable | infra lead | OCI image digest manifest for APIs, workers and frontends with build provenance report and scan result link | pending |
| migrations SQL frozen | data lead | SQL migration list, version report, dry-run output and rollback compatibility result for target commit | pending |
| latest rehearsal accepted | migration lead | accepted rehearsal reconciliation report, run evidence bundle, stability review and owner decision record | pending |
| rollback rehearsal accepted | infra lead | rollback rehearsal report with command output, measured duration result, restore target and owner decision record | pending |
| maintenance announced | support lead | customer announcement link, status page record, email preview, approval record and cutover journal reference | pending |
| secret inventory complete | security lead | secret owner list, production presence audit, rotation window, access review record and environment checksum | pending |
| dashboards open | infra lead | dashboard links for API, DB, queues, workers, edge, billing and audit with alert route test result | pending |
| incident channel open | migration lead | incident channel link, staffed owner rota, escalation record, support handoff and acknowledgement log | pending |

## Cutover Window

| Time | Action | Owner | Evidence | Status |
|---|---|---|---|---|
| T-0 | enable maintenance and read-only | infra lead | maintenance flag result, read-only command output, affected route list, owner acknowledgement and journal record | pending |
| T+15m | snapshot final source data | data lead | final snapshot id, checksum manifest, source LSN/timestamp, immutable storage record and retention approval | pending |
| T+30m | export final source data | data lead | export id, command output log, source row-count report, object checksum sample and export artifact link | pending |
| T+60m | transform and import | data lead | import run id, transform report, target row-count report, reject log and target version record | pending |
| T+90m | final reconciliation | data lead | production reconciliation report, checksum result, row-count delta report and reject disposition log | pending |
| T+105m | switch DNS/edge | infra lead | edge change id, DNS/TLS probe result, status-code snapshot, rollback route record and journal entry | pending |
| T+120m | start new workers | infra lead | worker health result, queue lag report, DLQ status record, topic offset snapshot and idempotency sample | pending |
| T+135m | run smoke tests | product leads | smoke report, request trace links, correlation IDs, runtime evidence bundle and owner sign-off record | pending |
| T+180m | go/no-go final | migration lead | signed go/no-go decision, journal entry, owner approval record, live evidence packet and communication decision record | pending |

## Smoke Tests

- login;
- MFA;
- workspace access;
- upload/download;
- share link;
- billing entitlement;
- webhook replay;
- privacy export;
- audit event;
- worker queue.

## Verification

```bash
pnpm check:migration-cutover-checklist -- --strict
```
