# Rollback Report

## Status

Template initialized. Rollback has not been rehearsed.

- `total_metadata_rows`: 8
- `none_metadata_rows`: 7
- `no_go_metadata_decisions`: 1
- `total_step_rows`: 6
- `pending_step_evidence_rows`: 0
- `pending_step_rows`: 0
- `passed_step_rows`: 0
- `accepted_step_rows`: 0
- `failed_step_rows`: 0
- `blocking_step_rows`: 6

## Rules

- A `go` decision requires all rollback metadata fields to be filled.
- A `go` decision requires parseable `started_at` and `completed_at`
  timestamps, with completion at or after start.
- A `go` decision requires measured `duration` to be less than or equal to
  `max_duration`.
- A `go` decision requires every rollback step to be `passed` or `accepted`.
- A `passed` or `accepted` rollback step requires concrete evidence such as an
  artifact, report, result, log, journal entry, incident record, snapshot,
  command output, health result, link or ID.

## Run Metadata

| Field | Value |
|---|---|
| environment | none |
| source_snapshot | none |
| target_commit | none |
| started_at | none |
| completed_at | none |
| duration | none |
| max_duration | none |
| decision | no-go until rehearsed |

## Steps

| Step | Evidence | Status |
|---|---|---|
| stop new public stack | command output, disabled route list, health result, edge metric snapshot and journal record showing public stack disabled | blocking |
| stop new workers | worker drain log, command output, queue lag report, DLQ status record and idempotency checkpoint showing workers stopped | blocking |
| restore DNS/edge to legacy | DNS and edge routing change record, TTL/probe result, TLS status, legacy health result and rollback route ID | blocking |
| restore legacy read-only or read-write | restore mode decision record, source snapshot health report, database access mode command output and owner approval record | blocking |
| preserve cutover logs | artifact archive link, log retention record, trace sample checksum, incident timeline export and immutable storage reference | blocking |
| open incident record | incident record ID, rollback trigger, owner assignment, first recovery action and rollback timeline journal link | blocking |

## Decision

Rollback is not approved until a rehearsal report replaces this template.

## Verification

```bash
pnpm check:migration-rollback-report -- --strict
```
