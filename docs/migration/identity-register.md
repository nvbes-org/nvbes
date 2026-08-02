# Identity Register Evidence

## Status

- status: passed
- checks: 26
- passed: 26
- failed: 0

## Rules

- Every evidence row must be generated from the identity register source contract.
- `passed` requires the configured file to contain the expected pattern.
- Summary counters must match evidence rows.
- Targeted tests must name the register checks required by parity.
- Generation provenance must identify sources, write command and targeted tests.

## Evidence

| Check | Status | Path |
|---|---:|---|
| Auth routes mount register under /auth | passed | `apps/identity-service/src/identity.domains.auth.routes.rs` |
| Register route accepts POST /register | passed | `apps/identity-service/src/identity.domains.auth.routes.register.rs` |
| Register route declares /auth/register OpenAPI path | passed | `apps/identity-service/src/identity.domains.auth.routes.register.rs` |
| Register route declares RegisterRequest body | passed | `apps/identity-service/src/identity.domains.auth.routes.register.rs` |
| Register route returns RegisterResult | passed | `apps/identity-service/src/identity.domains.auth.routes.register.rs` |
| Register route enforces proof-of-work when enabled | passed | `apps/identity-service/src/identity.domains.auth.routes.register.rs` |
| Register route applies dual rate limit | passed | `apps/identity-service/src/identity.domains.auth.routes.register.rs` |
| Register route resolves supported country to data region | passed | `apps/identity-service/src/identity.domains.auth.routes.register.rs` |
| Register route maps HTTP request to onboarding input through a tested helper | passed | `apps/identity-service/src/identity.domains.auth.routes.register.rs` |
| Register route mapping test covers data region, workspace, IP, and user-agent | passed | `apps/identity-service/src/identity.domains.auth.routes.register.tests.rs` |
| Onboarding exposes register command | passed | `apps/identity-service/src/identity.domains.auth.onboarding.rs` |
| Onboarding validates normalized email | passed | `apps/identity-service/src/identity.domains.auth.onboarding.rs` |
| Onboarding validates password policy | passed | `apps/identity-service/src/identity.domains.auth.onboarding.rs` |
| Onboarding applies account-level register rate limit | passed | `apps/identity-service/src/identity.domains.auth.onboarding.rs` |
| Onboarding creates the user account transaction | passed | `apps/identity-service/src/identity.domains.auth.onboarding.rs` |
| Onboarding records password history | passed | `apps/identity-service/src/identity.domains.auth.onboarding.rs` |
| Registration creates personal tenant | passed | `apps/identity-service/src/identity.domains.auth.db.account.rs` |
| Registration creates human principal | passed | `apps/identity-service/src/identity.domains.auth.db.account.rs` |
| Registration creates pending verification user | passed | `apps/identity-service/src/identity.domains.auth.db.account.rs` |
| Registration creates the tenant membership owned by Identity | passed | `apps/identity-service/src/identity.domains.auth.db.account.rs` |
| Registration writes user.registered audit event | passed | `apps/identity-service/src/identity.domains.auth.db.account.rs` |
| Registration issues verification email token | passed | `apps/identity-service/src/identity.domains.auth.db.account.rs` |
| Registration enqueues the verification email | passed | `apps/identity-service/src/identity.domains.auth.onboarding.rs` |
| OpenAPI source exports register path | passed | `apps/identity-service/src/identity.http.openapi.rs` |
| Generated OpenAPI contains /auth/register | passed | `apps/identity-service/openapi.json` |
| Generated OpenAPI exposes register operation id | passed | `apps/identity-service/openapi.json` |

## Decision

Identity register parity evidence is covered for repository cutover gates.

## Regeneration

```bash
pnpm check:migration-identity-register
node tools/migration/identity-register.mjs --write
```
