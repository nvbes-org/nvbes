# Smoke Test Manifest

## Status

Initialized. No critical smoke journey is approved for production cutover.

- `total_journey_rows`: 10
- `pending_journey_rows`: 10
- `passed_journey_rows`: 0
- `accepted_journey_rows`: 0
- `failed_journey_rows`: 0
- `blocking_journey_rows`: 0
- `go_decisions`: 0
- `no_go_decisions`: 10
- `total_runtime_evidence_rows`: 5
- `pending_runtime_evidence_rows`: 5
- `passed_runtime_evidence_rows`: 0
- `accepted_runtime_evidence_rows`: 0
- `failed_runtime_evidence_rows`: 0
- `blocking_runtime_evidence_rows`: 0

This manifest turns the runbook smoke list into explicit cutover evidence. A
smoke test is valid only when it has an owner, command, environment, evidence
link and signed `go` decision.

## Rules

- Every critical journey in the runbook must be represented here.
- Each journey needs a repeatable command or documented manual exception.
- A journey `go` decision requires command, environment, evidence and `passed`
  or `accepted` status.
- A journey `go` decision also requires every runtime evidence row to be
  `passed` or `accepted`.
- Runtime evidence status must be `pending`, `passed`, `accepted`, `failed` or
  `blocking`.
- Runtime evidence marked `passed` or `accepted` must name concrete artifact
  content such as commits, digests, versions, URLs, logs, request IDs,
  correlation IDs, metrics, reports or results.
- Production cutover requires the same manifest to pass in the final target
  environment after DNS/edge switch and worker start.
- `pending`, `unassigned`, missing evidence or `no-go` blocks cutover.
- The Support lead cannot announce service restored before this manifest and
  final reconciliation are both `go`.

## Critical Journeys

| Journey | Owner | Command | Environment | Evidence | Status | Decision |
|---|---|---|---|---|---|---|
| login | Identity owner | pnpm test:smoke -- --journey login --env production | final production target after DNS/edge switch | command log, request ID, session cookie proof and owner sign-off | pending | no-go |
| MFA | Identity owner | pnpm test:smoke -- --journey mfa --env production | final production target after DNS/edge switch | command log, challenge ID, AAL2 session proof and owner sign-off | pending | no-go |
| workspace access | Workspace/Authz owner | pnpm test:smoke -- --journey workspace-access --env production | final production target after DNS/edge switch | command log, tenant ID, role decision trace and owner sign-off | pending | no-go |
| upload/download | Drive owner | pnpm test:smoke -- --journey upload-download --env production | final production target after DNS/edge switch | command log, object checksum, download verification and owner sign-off | pending | no-go |
| share link | Drive owner | pnpm test:smoke -- --journey share-link --env production | final production target after DNS/edge switch | command log, share token, revoke verification and owner sign-off | pending | no-go |
| billing entitlement | Billing/Usage owner | pnpm test:smoke -- --journey billing-entitlement --env production | final production target after DNS/edge switch | command log, entitlement decision, usage ledger trace and owner sign-off | pending | no-go |
| webhook replay | Developer Platform owner | pnpm test:smoke -- --journey webhook-replay --env production | final production target after DNS/edge switch | command log, webhook delivery ID, idempotency result and owner sign-off | pending | no-go |
| privacy export | Audit/Privacy owner | pnpm test:smoke -- --journey privacy-export --env production | final production target after DNS/edge switch | command log, export artifact ID, retention class and owner sign-off | pending | no-go |
| audit event | Audit/Privacy owner | pnpm test:smoke -- --journey audit-event --env production | final production target after DNS/edge switch | command log, audit event ID, append-only verification and owner sign-off | pending | no-go |
| worker queue | Infra lead | pnpm test:smoke -- --journey worker-queue --env production | final production target after DNS/edge switch | command log, queue message ID, worker result and owner sign-off | pending | no-go |

## Runtime Evidence

| Evidence | Required content | Status |
|---|---|---|
| target version | deployment manifest with commit SHA, API/web/worker image digests, database migration versions, OpenAPI contract report and Protobuf contract report | pending |
| environment proof | environment artifact with public base URL, DNS target, edge route, worker image version, region placement and production configuration checksum | pending |
| logs | per-journey log artifact with request IDs, correlation IDs, tenant or workspace IDs where applicable, trace URLs and command exit records | pending |
| metrics | per-journey metrics report with API error rate, p95/p99 latency, queue lag, worker health, edge status codes and SLO snapshot link | pending |
| rollback still possible | rollback evidence packet with trigger window, restore target, last backup identifier, traffic rollback route and owner acknowledgement after smoke completion | pending |

## Decision

Cutover remains no-go until every critical journey and runtime evidence row is
signed with attached evidence and marked `go`.

## Verification

```bash
pnpm check:migration-smoke-tests -- --strict
```
