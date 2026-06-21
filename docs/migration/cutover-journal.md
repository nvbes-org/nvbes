# Cutover Journal

## Status

Template initialized. No production cutover has been executed.

- `total_metadata_rows`: 10
- `pending_metadata_rows`: 0
- `not_run_metadata_rows`: 1
- `not_scheduled_metadata_rows`: 1
- `metadata_go_decisions`: 0
- `metadata_no_go_decisions`: 1
- `total_timeline_rows`: 1
- `timeline_go_decisions`: 0
- `timeline_no_go_decisions`: 1
- `total_incident_rows`: 1
- `incident_no_go_rows`: 1
- `incident_resolved_rows`: 0
- `incident_accepted_rows`: 0
- `incident_failed_rows`: 0
- `incident_blocking_rows`: 0

## Rules

- The Migration lead owns this journal.
- Every critical action must be recorded with timestamp, owner, command or action,
  observed result and decision.
- Manual actions are no-go for future rehearsals unless they are backported into
  the cutover checklist.
- A rollback decision must include the exact trigger, timestamp, owner and first
  recovery action.
- A `go` journal decision requires every run metadata field to be filled.
- A `go` journal decision requires every timeline row to be `go` and every
  incident row to be `resolved` or `accepted`.
- A timeline entry marked `go` requires concrete time, owner, action, evidence
  and result.
- A timeline entry marked `go` requires a parseable timestamp.
- A timeline entry marked `go` requires concrete evidence and result references
  such as reports, logs, journal records, snapshots, commits, command output,
  reconciliation, smoke results, metrics, health checks, links or IDs.
- A resolved or accepted incident entry requires concrete time, severity, owner,
  symptom, impact and action.
- A resolved or accepted incident entry requires a parseable timestamp.
- A resolved or accepted incident entry requires concrete action evidence such
  as an incident record, journal entry, report, log, command output, link or ID.
- The journal is immutable after the post-migration audit starts; corrections are
  appended as new entries.

## Evidence Requirements

Run metadata evidence must include the signed target commit, release tag, image
digest manifest, build provenance record, final snapshot identifier, checksum
manifest, immutable storage reference, owner approval records and the staffed
cutover window roster. The decision row can move to `go` only after those
records are linked from the metadata table and the gate evidence packet shows
all G0-G8 gates as approved.

Timeline evidence must include the exact command or action executed, command
output or system event log, health or metric result, affected route or service
identifier, owner, timestamp and the related cutover checklist item. Data
movement timeline entries must also include source snapshot IDs, target import
run IDs, reconciliation reports, checksum bundles and reject-log references.
Traffic-shift timeline entries must include DNS, CDN, TLS, status-code and
synthetic probe results. Worker and async entries must include queue lag, DLQ,
schema registry and replay checkpoint evidence.

Incident evidence must include the incident record ID, severity, first symptom,
customer or internal impact, triggering metric or log sample, owner, first
recovery action, rollback trigger assessment and follow-up journal entry. A
resolved incident requires recovery proof such as health checks, smoke results,
metrics, reconciliation output or command logs. An accepted incident requires an
owner approval record, residual-risk entry and explicit post-cutover remediation
tracking.

## Run Metadata

| Field | Value |
|---|---|
| environment | not run |
| cutover window | not scheduled |
| target commit | signed target commit SHA and release tag record required |
| target images | immutable image digest manifest required |
| snapshot id | final snapshot id, checksum manifest and immutable storage record required |
| migration lead | named migration lead approval record required |
| data lead | named data lead approval record required |
| infra lead | named infra lead approval record required |
| support lead | named support lead approval record required |
| decision | no-go |

## Timeline

| Time | Phase | Owner | Action | Evidence | Result | Decision |
|---|---|---|---|---|---|---|
| not run | preparation | Migration lead | journal template initialized | this file | no production action | no-go |

## Incident Entries

| Time | Severity | Owner | Symptom | Impact | Action | Status |
|---|---|---|---|---|---|---|
| not run | none | Migration lead | no cutover executed | none | none | no-go |

## Decision

Current decision: no-go.

This journal cannot approve cutover until all G0-G8 gates are `go`, the
precutover gate passes, the final reconciliation passes and decision makers are
present in the cutover window.

## Verification

```bash
pnpm check:migration-cutover-journal -- --strict
```
