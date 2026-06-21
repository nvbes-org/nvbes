# Owner Sign-Off Matrix

## Status

Initialized. No production migration owner or gate sign-off is approved.

- total_signoff_rows: 31
- signed_rows: 0
- go_decisions: 0
- pending_rows: 31
- no_go_decisions: 31
- unassigned_owner_fields: 45

This matrix is the single human accountability index for the Big Bang migration.
Generated inventories can list routes, tables, secrets, jobs and resources, but
production cutover requires named owners with evidence links and explicit
decisions here.

## Rules

- Every role, domain and gate needs a primary owner and backup owner.
- Every source class needs a decision owner for accepts, rejects and rollback.
- A person cannot approve their own unchecked evidence for production cutover.
- A `go` decision requires `signed` status plus concrete owner evidence such
  as an artifact, report, result, generated ledger, journal entry, approval
  record, link, SHA, digest, snapshot, reconciliation, rollback or gate evidence.
- A gate sign-off can be `signed` or `go` only when the generated gate evidence
  has the same gate marked `go`.
- Required owner rows must name different primary and backup owners before they
  can be signed.
- Pending rows must remain `no-go`.
- Production requires `signed` status and `go` decision on every row.
- `pending`, `unassigned` or `no-go` anywhere in this file blocks cutover.

## Required Owners

| Area | Scope | Primary owner | Backup owner | Evidence | Status | Decision |
|---|---|---|---|---|---|---|
| Migration lead | sequence, journal, final go/no-go | unassigned | unassigned | cutover journal, gate evidence ledger and signed final decision record | pending | no-go |
| Data lead | snapshots, export, import, reconciliation | unassigned | unassigned | snapshot manifest, reconciliation report and reject disposition record | pending | no-go |
| Infra lead | deploy, DNS, backups, rollback | unassigned | unassigned | infra deploy evidence, backup/restore manifest and rollback report | pending | no-go |
| Security lead | secrets, access, audit, OSS export | unassigned | unassigned | secret map, access review record, audit evidence and OSS export result | pending | no-go |
| Support lead | maintenance, status page, hypercare | unassigned | unassigned | communication plan, status page record and hypercare support report | pending | no-go |
| Identity owner | auth, sessions, MFA, OAuth | unassigned | unassigned | identity login/session, MFA/WebAuthn and OAuth parity reports | pending | no-go |
| Workspace/Authz owner | tenants, roles, policies | unassigned | unassigned | workspace membership, last-owner and authorization policy reports | pending | no-go |
| Drive owner | metadata, storage, sharing, quotas | unassigned | unassigned | drive upload/download, share/revoke and quota evidence reports | pending | no-go |
| Billing/Usage owner | plans, entitlements, ledger, webhooks | unassigned | unassigned | billing entitlement and webhook idempotency evidence reports | pending | no-go |
| Audit/Privacy owner | audit log, exports, deletion, retention | unassigned | unassigned | audit append-only, privacy export/delete and retention evidence reports | pending | no-go |
| Developer Platform owner | apps, tokens, webhooks, SDKs | unassigned | unassigned | developer OAuth/token and signed webhook evidence reports | pending | no-go |
| Cloud/Internal owner | provisioning, support, operations | unassigned | unassigned | cloud provisioning and infra deploy evidence reports | pending | no-go |
| Contracts owner | OpenAPI, Protobuf, event schemas | unassigned | unassigned | generated OpenAPI, Protobuf and event schema contract reports | pending | no-go |
| OSS export owner | public allowlist and private-doc exclusion | unassigned | unassigned | OSS export manifest, provider boundary report and approval record | pending | no-go |

## Source Class Sign-Offs

| Source class | Decision owner | Evidence | Status | Decision |
|---|---|---|---|---|
| routes and endpoints | unassigned | inventory generated report and OpenAPI parity matrix | pending | no-go |
| source tables | unassigned | data map generated report and reconciliation coverage record | pending | no-go |
| secrets | unassigned | secret map generated report and owner access review record | pending | no-go |
| jobs and workers | unassigned | job map generated report and worker replay evidence | pending | no-go |
| infra resources | unassigned | resource map generated report and deployment inventory record | pending | no-go |
| blocking risks | unassigned | risk register generated report and live evidence blocker record | pending | no-go |
| rejected data | unassigned | reject log, disposition report and owner acceptance record | pending | no-go |
| rollback path | unassigned | rollback report, rehearsal ledger and timed restore evidence | pending | no-go |

## Gate Sign-Offs

| Gate | Owner | Evidence | Status | Decision |
|---|---|---|---|---|
| G0 Freeze | unassigned | gate evidence report and freeze approval record | pending | no-go |
| G1 Fondation | unassigned | gate evidence report and runtime foundation approval record | pending | no-go |
| G2 Primitives | unassigned | gate evidence report and platform primitives approval record | pending | no-go |
| G3 Domaines | unassigned | gate evidence report and domain parity approval record | pending | no-go |
| G4 Frontends | unassigned | gate evidence report and frontend smoke approval record | pending | no-go |
| G5 Infra | unassigned | gate evidence report and infra restore/rollback approval record | pending | no-go |
| G6 Repetitions | unassigned | gate evidence report and rehearsal approval record | pending | no-go |
| G7 Cutover | unassigned | gate evidence report and live cutover approval record | pending | no-go |
| G8 Decommission | unassigned | gate evidence report and decommission audit approval record | pending | no-go |

## Decision

No production cutover can be approved until all required owners, source classes
and gates are signed with evidence and marked `go`.

## Verification

```bash
pnpm check:migration-owner-signoffs -- --strict
```
