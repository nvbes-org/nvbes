# Migration Rehearsal Ledger

## Status

Template initialized. No migration rehearsal is approved.

- `total_run_rows`: 3
- `pending_run_rows`: 3
- `passed_run_rows`: 0
- `accepted_run_rows`: 0
- `failed_run_rows`: 0
- `blocking_run_rows`: 0
- `run_go_decisions`: 0
- `run_no_go_decisions`: 3
- `total_evidence_cells`: 21
- `pending_evidence_cells`: 0
- `total_stability_rows`: 5
- `stability_go_decisions`: 0
- `stability_no_go_decisions`: 5

## Rules

- At least three rehearsals are required before production cutover:
  dry run local, dry run staging with anonymized data and dress rehearsal on a
  production snapshot.
- Every rehearsal must reference an immutable source snapshot, target commit,
  image set, migration scripts and contract versions.
- A rehearsal is invalid if it uses a manual action that is not added to the
  cutover checklist.
- Two consecutive rehearsals must have stable reconciliation before G6 can move
  to `go`.
- A required run can move to `go` only when its status is `passed` or
  `accepted`, its source snapshot and target version are concrete, and required
  evidence is not pending.
- A required run can move to `go` only when every evidence type listed in
  `Required Evidence` has a concrete artifact reference in `Run Evidence`.
- A required run `go` decision requires concrete source snapshot and target
  version references.
- Run evidence cells must reference immutable artifacts. Bare `passed` or
  `accepted` markers are invalid because they do not identify the proof.
- A stability review can move to `go` only when the current state is concrete;
  pending stability states must remain `no-go`.
- Stability `go` current state must reference concrete proof such as a report,
  result, reconciliation, rollback, checklist entry, link or record.
- The Data lead owns data results. The Migration lead owns the final rehearsal
  decision.

## Evidence Requirements

Each rehearsal report must be a replayable packet, not a narrative summary. The
packet must include:

- source snapshot manifest, checksum bundle, immutable storage reference and
  source row-count report;
- signed target commit, release tag, image digest manifest, migration script
  versions and generated contract versions;
- export, transform and import command transcripts with operator, timestamp,
  environment, exit code and artifact references;
- reconciliation JSON, checksum comparison, row-count delta report, reject log
  snapshot and owner disposition for every non-zero delta;
- observability bundle containing dashboard snapshot, alert status, request
  traces, correlation IDs and service health metrics captured during the run;
- manual-action log proving every human step is either absent or backported into
  the cutover checklist with an owner and command replacement;
- owner sign-off records from Data lead and Migration lead, including the run
  identifier, scope, decision and immutable artifact links.

R2 and R3 additionally require rollback proof with command output, start and end
timestamps, measured duration, restored target identifier, data integrity check,
queue/DLQ state, DNS or edge rollback evidence when applicable and comparison
against the announced rollback window.

R3 additionally requires smoke-test proof for the production-snapshot target:
journey result, request trace, status code, audit/event side effect, dashboard
metric and owner approval for every critical customer path.

## Required Runs

| Run | Environment | Source Snapshot | Target Version | Required Evidence | Status | Decision |
|---|---|---|---|---|---|---|
| R1 | local | local seeded source snapshot manifest and checksum | target commit SHA, image digests, migration IDs and contract versions | export, transform, import, reconciliation | pending | no-go |
| R2 | staging anonymized | anonymized staging source snapshot manifest, checksum and masking report | target commit SHA, image digests, migration IDs and contract versions | export, transform, import, reconciliation, rollback | pending | no-go |
| R3 | production snapshot dress rehearsal | production snapshot manifest, checksum and immutable storage reference | target commit SHA, image digests, migration IDs and contract versions | export, transform, import, reconciliation, smoke, rollback | pending | no-go |

## Run Evidence

| Run | Export | Transform | Import | Reconciliation | Smoke | Rollback | Owner Sign-Off |
|---|---|---|---|---|---|---|---|
| R1 | export command output log and source row-count report | transform command output log and mapping diff report | import command output log and target row-count report | reconciliation JSON report and checksum result | smoke not required for R1; record manual exception in run report | rollback not required for R1; record manual exception in run report | Data lead and Migration lead sign-off record |
| R2 | export command output log and anonymized source row-count report | transform command output log and masking validation report | import command output log and target row-count report | reconciliation JSON report and checksum result | smoke not required for R2; record manual exception in run report | rollback command output log and duration result | Data lead and Migration lead sign-off record |
| R3 | export command output log and production snapshot row-count report | transform command output log and mapping diff report | import command output log and target row-count report | reconciliation JSON report and checksum result | smoke-test manifest result and request trace log | rollback command output log and duration result | Data lead and Migration lead sign-off record |

## Stability Review

| Check | Required State | Current State | Decision |
|---|---|---|---|
| blocking rejects | zero or owner accepted | reconciliation report result proving zero blocking rejects or owner acceptance record | no-go |
| row count deltas | zero or owner accepted | reconciliation row-count delta report with owner acceptance record for any non-zero delta | no-go |
| checksum mismatch | zero or owner accepted | checksum comparison report with owner acceptance record for any mismatch | no-go |
| rollback duration | inside announced window | rollback duration result and command output log | no-go |
| manual actions | all added to cutover checklist | cutover checklist diff and manual-action owner record | no-go |

## Decision

Current decision: no-go.

G6 cannot pass until this ledger references three completed rehearsal reports,
two consecutive reconciliations are stable and rollback evidence is attached.

## Verification

```bash
pnpm check:migration-rehearsals -- --strict
```
