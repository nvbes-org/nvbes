# Decommission Manifest

## Status

Initialized. Decommission is not approved.

- `total_decision_rows`: 16
- `pending_rows`: 16
- `passed_rows`: 0
- `accepted_rows`: 0
- `removed_rows`: 0
- `failed_rows`: 0
- `blocking_rows`: 0
- `go_decisions`: 0
- `no_go_decisions`: 16

This manifest captures the final zero-debt requirement after cutover: no legacy
runtime service, endpoint, job, secret, script, config or private export remains
active or required for V1 operation.

## Rules

- Every legacy runtime component must be removed, archived read-only or signed
  as intentionally retained outside runtime.
- Every legacy secret must be revoked or rotated with evidence.
- Every migration export must have retention, encryption and expiry evidence.
- Public and private runbooks must be current before final approval.
- A `go` decision requires `passed`, `accepted` or `removed` status; runtime rows
  also require concrete removal or archive evidence such as an inventory,
  report, log, record, manifest, retention proof, link, ID or check result.
- A final evidence `go` decision requires every legacy runtime row to be
  `passed`, `accepted` or `removed`.
- A final evidence `go` decision requires an owner and concrete required
  content such as a log, report, result, retention record, docs link, audit
  report, debt review or incident list.
- `pending`, missing evidence or `no-go` blocks goal completion.

## Legacy Runtime

| Component | Owner | Evidence | Status | Decision |
|---|---|---|---|---|
| legacy API services | Infra lead | service inventory report, removal command log, disabled endpoint check result, traffic metric report and rollback archive record | pending | no-go |
| legacy web frontends | Product leads | frontend inventory report, archived deployment manifest, disabled route check result, CDN purge log and owner approval record | pending | no-go |
| legacy workers | Infra lead | worker inventory report, disabled runtime log, queue drain record, scheduler detach check and idempotency retention record | pending | no-go |
| legacy queues | Infra lead | queue inventory report, drain/deletion record, DLQ retention manifest, topic offset archive and consumer disable log | pending | no-go |
| legacy schedules | Infra lead | schedule inventory report, disabled cron log, scheduler audit record, next-run absence check and owner approval record | pending | no-go |
| legacy storage paths | Data lead | storage path inventory report, retention manifest, deletion/hold record, encryption proof and sampled access-denied check result | pending | no-go |
| legacy configs | Infra lead | config inventory report, deleted legacy config record, rollback-safe archive link, environment diff report and owner approval record | pending | no-go |
| legacy scripts | Migration lead | script inventory report, archived migration script manifest, read-only retention record, executable-bit removal log and docs link update | pending | no-go |

## Final Evidence

| Evidence | Owner | Required content | Status | Decision |
|---|---|---|---|---|
| legacy secrets revoked | Security lead | rotation or revocation log, access review result, secret inventory report, environment checksum and post-revocation probe result | pending | no-go |
| OSS export clean | Security lead | `pnpm check:oss-export` result, export manifest, private-doc scan report and signed Security review record | pending | no-go |
| boundaries clean | Migration lead | `pnpm check:oss-boundaries` result, provider boundary report, dependency graph snapshot and zero-violation owner record | pending | no-go |
| migration exports archived | Data lead | retention report, encryption evidence, expiry record, immutable archive reference and deletion/hold approval path | pending | no-go |
| support hypercare closed | Support lead | support report, open incident list, owner handoff log, customer communication record and residual-risk acceptance | pending | no-go |
| runbooks updated | Migration lead | public/private docs links, runbook update report, outdated-link check result and owner acknowledgement record | pending | no-go |
| V2 backlog clean | Product leads | debt review report proving no V1-required debt item remains, backlog query result and product owner acceptance record | pending | no-go |
| post-migration audit signed | Migration lead | approved audit report, sign-off record, audit item evidence bundle and final decommission decision link | pending | no-go |

## Decision

The migration goal remains incomplete until every decommission row is evidenced,
signed and marked `go`.

## Verification

```bash
pnpm check:migration-decommission -- --strict
```
