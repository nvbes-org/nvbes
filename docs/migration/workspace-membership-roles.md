# Workspace Membership Roles Evidence

## Status

- status: passed
- checks: 26
- passed: 26
- failed: 0

## Rules

- Every evidence row must be generated from the workspace membership source contract.
- `passed` requires the configured source or OpenAPI document to contain the expected pattern.
- Summary counters must match evidence rows.
- Targeted tests must name role-boundary and denied-audit metadata checks required by parity.
- Generation provenance must identify sources, write command and targeted tests.

## Evidence

| Check | Status | Path |
|---|---:|---|
| Owner role is part of the workspace role model | passed | `libs/rust/core/src/authz.role.rs` |
| Admin role is part of the workspace role model | passed | `libs/rust/core/src/authz.role.rs` |
| Security admin role is part of the workspace role model | passed | `libs/rust/core/src/authz.role.rs` |
| Billing admin role is part of the workspace role model | passed | `libs/rust/core/src/authz.role.rs` |
| Member role is part of the workspace role model | passed | `libs/rust/core/src/authz.role.rs` |
| Viewer role is part of the workspace role model | passed | `libs/rust/core/src/authz.role.rs` |
| Workspace role policy uses the shared is_allowed entrypoint | passed | `libs/rust/core/src/authz.policy.rs` |
| Owner policy is explicit | passed | `libs/rust/core/src/authz.policy.rs` |
| Admin policy is explicit | passed | `libs/rust/core/src/authz.policy.rs` |
| Security admin policy is explicit | passed | `libs/rust/core/src/authz.policy.rs` |
| Billing admin policy is explicit | passed | `libs/rust/core/src/authz.policy.rs` |
| Member policy is explicit | passed | `libs/rust/core/src/authz.policy.rs` |
| Viewer policy is explicit | passed | `libs/rust/core/src/authz.policy.rs` |
| Core test covers owner/admin/security/billing/member/viewer permission boundaries | passed | `libs/rust/core/src/authz.policy.tests.rs` |
| Core test blocks admin escalation through invites | passed | `libs/rust/core/src/authz.policy.tests.rs` |
| Core test limits security admin to audit/security views | passed | `libs/rust/core/src/authz.policy.tests.rs` |
| Core test limits billing admin away from audit | passed | `libs/rust/core/src/authz.policy.tests.rs` |
| Core test keeps viewer read-only | passed | `libs/rust/core/src/authz.policy.tests.rs` |
| Identity API authorization calls the shared role policy | passed | `apps/identity-api/src/identity.domains.authz.service.rs` |
| Denied workspace actions are audited before returning an error | passed | `apps/identity-api/src/identity.domains.authz.service.rs` |
| Permission denied decisions insert an audit event | passed | `apps/identity-api/src/identity.domains.authz.db.rs` |
| Permission denied audit uses stable action literal | passed | `apps/identity-api/src/identity.domains.authz.db.rs` |
| Permission denied audit metadata is generated through a tested helper | passed | `apps/identity-api/src/identity.domains.authz.db.rs` |
| Permission denied audit metadata has a unit test | passed | `apps/identity-api/src/identity.domains.authz.db.rs` |
| Authz decision endpoint is routed | passed | `apps/identity-api/src/identity.domains.authz.routes.rs` |
| Authz decision endpoint is present in OpenAPI | passed | `apps/identity-api/openapi.json` |

## Decision

Workspace/Authz membership role parity evidence is covered for repository cutover gates.

## Regeneration

```bash
pnpm check:migration-workspace-membership-roles
tools/migration/workspace-membership-roles.mjs --write
```
