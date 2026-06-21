# Migration Communication Plan

## Status

Template initialized. No production communication has been approved.

- `total_audience_rows`: 4
- `pending_audience_rows`: 4
- `ready_audience_rows`: 0
- `sent_audience_rows`: 0
- `accepted_audience_rows`: 0
- `failed_audience_rows`: 0
- `blocking_audience_rows`: 0
- `total_message_rows`: 6
- `go_decisions`: 0
- `no_go_decisions`: 6
- `total_approval_rows`: 5
- `pending_approval_rows`: 5
- `ready_approval_rows`: 0
- `passed_approval_rows`: 0
- `accepted_approval_rows`: 0

## Rules

- Support lead owns customer-facing communication.
- Migration lead approves every status update before publication.
- No success message may be sent before smoke tests and final reconciliation pass.
- Rollback communication must be ready before the cutover window starts.
- Audience status must be `pending`, `ready`, `sent`, `accepted`, `failed` or
  `blocking`; ready/sent/accepted rows require channel, owner and timing.
- Message `go` decisions require trigger, approver and concrete evidence such
  as a link, URL, status page entry, sent message, approval record, cutover
  journal entry, incident route, report or result.
- Message `go` decisions require every approval checklist row to be `passed` or
  `accepted`.
- Approval checklist current state must stay `pending` or be `ready`, `passed`
  or `accepted`; `passed` and `accepted` rows require concrete required-state
  evidence such as verified links, tested routes, approvals, success criteria
  or reconciliation results.
- Every published message must be linked from the cutover journal.

## Audiences

| Audience | Channel | Owner | Required Timing | Status |
|---|---|---|---|---|
| customers | status page and email | Support lead | before T-7j | pending |
| internal responders | incident channel | Migration lead | before T-2h | pending |
| product owners | cutover channel | Migration lead | before T-2h | pending |
| support team | support brief | Support lead | before T-24h | pending |

## Message Templates

| Message | Trigger | Approver | Evidence | Decision |
|---|---|---|---|---|
| maintenance announcement | cutover scheduled | Migration lead | approved status page draft, customer email preview and cutover journal record | no-go |
| maintenance started | T-0 read-only enabled | Migration lead | status page update record, read-only command result and cutover journal entry | no-go |
| progress update | checkpoint delay or milestone | Migration lead | approved message template, milestone report and cutover journal entry | no-go |
| rollback notice | rollback decision made | Migration lead | approved rollback message, incident route and rollback decision record | no-go |
| service restored | smoke and reconciliation passed | Migration lead | smoke-test result, reconciliation report and status page restoration record | no-go |
| post-cutover hypercare | service restored | Support lead | support brief, hypercare owner rota and open incident report | no-go |

## Approval Checklist

| Check | Required State | Current State |
|---|---|---|
| status page ready | verified status page URL, maintenance draft link, restoration draft link and owner approval record | pending |
| customer impact described | approved customer impact statement with affected products, read-only window, expected duration and support contact route | pending |
| rollback message ready | approved rollback message artifact with trigger criteria, incident route, customer-facing wording and migration lead sign-off before cutover | pending |
| support escalation route | staffed support escalation route with on-call rota, incident channel URL, tested handoff record and acknowledgement from support lead | pending |
| final success criteria | signed success criteria record requiring smoke-test result, reconciliation report, observability snapshot and cutover journal entry | pending |

## Decision

Current decision: no-go.

Communication is not approved until the cutover window, customer impact,
rollback wording, status page and decision owners are attached as evidence.

## Verification

```bash
pnpm check:migration-communication -- --strict
```
