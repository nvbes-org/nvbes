# Release Freeze Manifest

## Status

Initialized. No target release is frozen for production cutover.

- `total_decision_rows`: 13
- `pending_rows`: 13
- `passed_rows`: 0
- `accepted_rows`: 0
- `failed_rows`: 0
- `blocking_rows`: 0
- `go_decisions`: 0
- `no_go_decisions`: 13

This manifest captures the immutable target version required by the runbook:
commit, image digests, SQL migrations, contracts, capacity, secrets and freeze
decisions.

## Rules

- Every release input must be immutable and reproducible before cutover.
- Image references must use digests, not mutable tags.
- SQL migrations and contracts must match the exact target commit.
- Capacity, production secrets, deploy freeze and billing freeze must be signed.
- A `go` row requires immutable concrete evidence and `passed` or `accepted`
  status; accepted evidence must name concrete references such as commit SHAs,
  digests, manifests, migration reports, contracts, schemas, signed records,
  capacity reports, secret records or rollback targets.
- An operational freeze `go` requires every release input row to be marked `go`.
- `pending`, `failed` or `blocking` status requires a `no-go` decision.
- `pending`, missing evidence or `no-go` blocks production cutover.

## Release Inputs

| Input | Owner | Evidence | Status | Decision |
|---|---|---|---|---|
| target commit | Migration lead | signed target commit SHA, annotated repository tag, CI run URL and clean source archive checksum record | pending | no-go |
| Rust API images | Infra lead | immutable OCI image digest manifest for every Rust API service, build provenance report and vulnerability scan result | pending | no-go |
| worker images | Infra lead | immutable OCI image digest manifest for identity and drive workers, build provenance report and queue compatibility result | pending | no-go |
| frontend artifacts | Product leads | signed frontend artifact manifest with commit SHA, content digests, route manifest checksum and CDN publish preview record | pending | no-go |
| SQL migrations | Data lead | migration version report matching target commit, database target list, dry-run output and rollback compatibility record | pending | no-go |
| OpenAPI contracts | Migration lead | generated OpenAPI contract artifact with checksum, target commit, SDK generation result and breaking-change report | pending | no-go |
| Protobuf contracts | Migration lead | generated Protobuf descriptor artifact with checksum, target commit, codegen result and compatibility report | pending | no-go |
| event schemas | Migration lead | schema registry export artifact with checksums, target commit, compatibility report and replay contract result | pending | no-go |

## Operational Freezes

| Freeze | Owner | Evidence | Status | Decision |
|---|---|---|---|---|
| capacity verified | Infra lead | signed capacity report covering API, workers, database, queues, edge and billing paths with load-test result and headroom calculation | pending | no-go |
| production secrets present | Security lead | secret presence audit record with required key list, rotation window, access review, environment checksum and owner sign-off | pending | no-go |
| deploy freeze active | Migration lead | signed deploy freeze record with frozen target version, allowed emergency change policy, approver list and audit log link | pending | no-go |
| billing mutation freeze | Billing/Usage owner | signed billing mutation freeze record with disabled mutation list, webhook replay policy, ledger snapshot and owner acknowledgement | pending | no-go |
| rollback target confirmed | Infra lead | rollback target version, image digest manifest, restore record, traffic rollback route and measured rollback window report | pending | no-go |

## Decision

Cutover remains no-go until every release input and operational freeze row is
evidenced, signed and marked `go`.

## Verification

```bash
pnpm check:migration-release-freeze -- --strict
```
