# Observability Readiness

## Status

Initialized. Production cutover is no-go until observability evidence is signed.

- `total_decision_rows`: 13
- `pending_rows`: 13
- `passed_rows`: 0
- `accepted_rows`: 0
- `failed_rows`: 0
- `blocking_rows`: 0
- `go_decisions`: 0
- `no_go_decisions`: 13

This manifest covers the runbook requirement that logs, traces, metrics,
dashboards, alerts and SLO signals are visible before final go/no-go.

## Rules

- Every critical signal needs an owner, evidence link and tested alert path.
- A critical signal `go` decision requires owner, evidence, alert path and
  `passed` or `accepted` status.
- A critical signal `go` decision also requires every cutover evidence row to be
  `passed` or `accepted`.
- Cutover evidence `go`, `passed` or `accepted` rows require concrete artifact
  content such as dashboards, alert records, incidents, logs, traces, samples,
  snapshots, reports, results, links or metrics.
- Dashboards must cover API, database, queues, workers, edge and billing.
- Logs and traces must expose request IDs and correlation IDs.
- Production cutover cannot proceed with blind spots on auth, data or billing.
- `pending`, missing evidence or `no-go` blocks cutover.

## Critical Signals

| Signal | Scope | Owner | Evidence | Alert path | Status | Decision |
|---|---|---|---|---|---|---|
| API health | errors, latency, saturation | Infra lead | API dashboard link, error budget snapshot and p95/p99 latency panel | routed 5xx and saturation alert with acknowledged incident record | pending | no-go |
| auth health | login, MFA, token refresh | Identity owner | login, MFA and token-refresh dashboard link with smoke trace sample | routed auth failure alert with acknowledged incident record | pending | no-go |
| database health | connections, locks, replication, migrations | Data lead | database dashboard link, replication snapshot and migration health report | routed connection, lock and replication alert with acknowledged incident record | pending | no-go |
| queue health | lag, DLQ, worker failures | Infra lead | queue dashboard link, lag snapshot and DLQ report | routed lag and DLQ alert with acknowledged incident record | pending | no-go |
| worker health | job success, retry, idempotency | Infra lead | worker dashboard link, retry sample and idempotency result report | routed worker failure alert with acknowledged incident record | pending | no-go |
| edge health | DNS, TLS, routing, status codes | Infra lead | edge dashboard link, DNS/TLS probe report and status-code snapshot | routed edge error alert with acknowledged incident record | pending | no-go |
| billing health | entitlements, ledger, webhook replay | Billing/Usage owner | billing dashboard link, entitlement sample and webhook replay result | routed billing replay or ledger drift alert with acknowledged incident record | pending | no-go |
| audit health | append-only writes and export visibility | Security lead | audit dashboard link, append-only write sample and export trace report | routed audit write/export alert with acknowledged incident record | pending | no-go |

## Cutover Evidence

| Evidence | Required content | Status | Decision |
|---|---|---|---|
| dashboard set | dashboard URLs or exported dashboard artifact covering API, database, queues, workers, edge, billing and audit panels with owner review record | pending | no-go |
| alert test | fired alert artifact, routed incident URL, notification delivery log and acknowledged owner record for every critical alert path | pending | no-go |
| log trace sample | log and trace sample artifacts for every smoke journey, including request ID, correlation ID, tenant ID where applicable and linked trace URL | pending | no-go |
| SLO snapshot | SLO snapshot report with API error rate, p95/p99 latency, queue lag, worker health, edge status codes, billing replay drift and audit write success metrics | pending | no-go |
| incident channel | staffed incident channel URL, escalation route artifact, on-call owner acknowledgement and support handoff record for the cutover window | pending | no-go |

## Decision

Cutover remains no-go until every critical signal and cutover evidence row is
signed with proof and marked `go`.

## Verification

```bash
pnpm check:migration-observability -- --strict
```
