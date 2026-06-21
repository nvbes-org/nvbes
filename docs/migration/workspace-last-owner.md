# Workspace Last-Owner Evidence

## Status

- status: passed
- checks: 18
- passed: 18
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
| Owner changes use tenant-scoped advisory transaction lock | passed | `apps/identity-api/src/identity.domains.enterprise.db.owners.rs` |
| Role updates count workspaces that would lose their last owner | passed | `apps/identity-api/src/identity.domains.enterprise.db.owners.rs` |
| Lifecycle updates count workspaces that would lose their last owner | passed | `apps/identity-api/src/identity.domains.enterprise.db.owners.rs` |
| Ownerless queries require absence of another active owner | passed | `apps/identity-api/src/identity.domains.enterprise.db.owners.rs` |
| Enterprise access updates lock owner changes before mutation | passed | `apps/identity-api/src/identity.domains.enterprise.service.user_mutations.rs` |
| Enterprise access updates check ownerless workspaces before replace | passed | `apps/identity-api/src/identity.domains.enterprise.service.user_mutations.rs` |
| Enterprise access updates reject last-owner removal | passed | `apps/identity-api/src/identity.domains.enterprise.service.user_mutations.rs` |
| Enterprise access updates audit membership changes | passed | `apps/identity-api/src/identity.domains.enterprise.service.user_mutations.rs` |
| Enterprise suspension checks ownerless workspaces before status change | passed | `apps/identity-api/src/identity.domains.enterprise.service.user_mutations.rs` |
| Enterprise suspension audits membership changes | passed | `apps/identity-api/src/identity.domains.enterprise.service.user_mutations.rs` |
| Access review member revocation locks owner changes | passed | `apps/identity-api/src/identity.domains.enterprise.access_reviews.revocations.rs` |
| Access review member revocation checks tenant last owner | passed | `apps/identity-api/src/identity.domains.enterprise.access_reviews.revocations.rs` |
| Access review role revocation checks workspace last owner | passed | `apps/identity-api/src/identity.domains.enterprise.access_reviews.revocations.rs` |
| Access review service-account revocation checks workspace last owner | passed | `apps/identity-api/src/identity.domains.enterprise.access_reviews.revocations.rs` |
| Last-owner service guard returns stable conflict code | passed | `apps/identity-api/src/identity.domains.enterprise.access_reviews.revocations.rs` |
| Service test allows non-ownerless mutation | passed | `apps/identity-api/src/identity.domains.enterprise.access_reviews.revocations.tests.rs` |
| Service test blocks ownerless mutation | passed | `apps/identity-api/src/identity.domains.enterprise.access_reviews.revocations.tests.rs` |
| Core authz test prevents owner-to-owner invite/remove actions | passed | `libs/rust/core/src/authz.policy.tests.rs` |

## Decision

Workspace/Authz last-owner protection evidence is covered for repository cutover gates.

## Regeneration

```bash
pnpm check:migration-workspace-last-owner
tools/migration/workspace-last-owner.mjs --write
```
