# Migration Rejects

## Status

Initialized. No reject has been accepted for cutover.

- `total_reject_rows`: 1
- `blocking_reject_rows`: 1
- `accepted_reject_rows`: 0
- `fixed_reject_rows`: 0
- `no_go_decisions`: 1
- `accepted_decisions`: 0
- `fixed_decisions`: 0

## Rules

- Every rejected row or object needs a class, owner and impact.
- `blocking` rejects stop cutover.
- `accepted` rejects require owner approval, documented impact and immutable
  evidence such as an artifact, report, result, log, journal entry, approval
  record, reconciliation, checksum, snapshot, run ID or link.
- `fixed` rejects require concrete evidence of the correction such as a result,
  diff, patch, report, run ID or link.
- Reject class and final decision must match: `accepted` rejects need an
  `accepted` decision, and `fixed` rejects need a `fixed` decision.

## Evidence Bundle Requirements

Every reject disposition must carry enough proof for an independent reviewer to
replay the decision without relying on chat history or tribal knowledge.

Accepted reject evidence must include:

- the run identifier, source system, domain and object identifier;
- the business impact statement approved by the accountable owner;
- the owner approval record with approver identity, timestamp and scope;
- the immutable artifact reference for the source snapshot or report;
- the reconciliation, checksum, query result or log proving the rejected value;
- the cutover journal entry that records why the risk is accepted;
- the follow-up risk register entry when remediation remains after cutover.

Fixed reject evidence must include:

- the failing input artifact or report that produced the reject;
- the corrective patch, migration transform, command output or data repair log;
- the rerun identifier for the validator that now passes;
- the before/after reconciliation, checksum or row-count comparison;
- the owner review record confirming the correction is complete;
- the cutover journal entry that links the fix to the final decision.

Unresolved reject evidence must include:

- the missing proof item that prevents acceptance or fix confirmation;
- the owner currently accountable for producing that proof;
- the next command, report or reconciliation run required to move the reject;
- the explicit gate or cutover checklist item held by the reject.

## Reject Log

| Run | Domain | Source | Identifier | Class | Owner | Impact | Evidence | Decision |
|---|---|---|---|---|---|---|---|---|
| none | none | none | none | blocking | migration lead | no run executed | none | no-go |

## Verification

```bash
pnpm check:migration-rejects -- --strict
```
