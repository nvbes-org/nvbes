# Workspace Last-Owner Evidence

## Status

- status: passed
- checks: 17
- passed: 17
- failed: 0

## Rules

- Every evidence row must be generated from the workspace last-owner source contract.
- `passed` requires the configured source file to contain the expected pattern.
- Summary counters must match evidence rows.
- Targeted tests must name last-owner guard and owner policy checks required by parity.
- Generation provenance must identify sources, write command and targeted tests.

## Evidence

| Check | Status | Path |
|---|---:|---|
| Owner changes lock the tenant row before mutation | passed | `apps/enterprise-service/src/enterprise.grpc.user_access.db.rs` |
| Role updates count workspaces that would lose their last owner | passed | `apps/enterprise-service/src/enterprise.grpc.user_access.db.rs` |
| Lifecycle updates count workspaces that would lose their last owner | passed | `apps/enterprise-service/src/enterprise.grpc.user_access.db.rs` |
| Ownerless queries require absence of another active owner | passed | `apps/enterprise-service/src/enterprise.grpc.user_access.db.rs` |
| Enterprise access updates lock owner changes before mutation | passed | `apps/enterprise-service/src/enterprise.grpc.user_access.rs` |
| Enterprise access updates check ownerless workspaces before replace | passed | `apps/enterprise-service/src/enterprise.grpc.user_access.rs` |
| Enterprise access updates reject last-owner removal with failed-precondition | passed | `apps/enterprise-service/src/enterprise.grpc.user_access.db.rs` |
| Enterprise access updates audit membership changes | passed | `apps/enterprise-service/src/enterprise.grpc.user_access.rs` |
| Enterprise suspension checks ownerless workspaces before status change | passed | `apps/enterprise-service/src/enterprise.grpc.user_access.rs` |
| Enterprise suspension audits membership changes | passed | `apps/enterprise-service/src/enterprise.grpc.user_access.rs` |
| Access review member revocation checks tenant last owner | passed | `apps/enterprise-service/src/enterprise.grpc.access_reviews.revocations.rs` |
| Access review role revocation checks workspace last owner | passed | `apps/enterprise-service/src/enterprise.grpc.access_reviews.revocations.rs` |
| Access review service-account revocation checks workspace last owner | passed | `apps/enterprise-service/src/enterprise.grpc.access_reviews.revocations.rs` |
| Last-owner service guard returns stable failed-precondition status | passed | `apps/enterprise-service/src/enterprise.grpc.access_reviews.revocations.rs` |
| Service test allows non-ownerless mutation | passed | `apps/enterprise-service/src/enterprise.grpc.access_reviews.revocations.rs` |
| Service test blocks ownerless mutation | passed | `apps/enterprise-service/src/enterprise.grpc.access_reviews.revocations.rs` |
| Core authz test prevents owner-to-owner invite/remove actions | passed | `libs/rust/core/src/authz.policy.tests.rs` |

## Decision

Workspace/Authz last-owner protection evidence is covered for repository cutover gates.

## Regeneration

```bash
pnpm check:migration-workspace-last-owner
node tools/migration/workspace-last-owner.mjs --write
```
