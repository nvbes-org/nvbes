# Identity Register Evidence

## Status

- status: passed
- checks: 29
- passed: 29
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
| Auth routes mount register under /auth | passed | `apps/identity-api/src/identity.domains.auth.routes.rs` |
| Register route accepts POST /register | passed | `apps/identity-api/src/identity.domains.auth.routes.register.rs` |
| Register route declares /auth/register OpenAPI path | passed | `apps/identity-api/src/identity.domains.auth.routes.register.rs` |
| Register route declares RegisterRequest body | passed | `apps/identity-api/src/identity.domains.auth.routes.register.rs` |
| Register route returns RegisterResult | passed | `apps/identity-api/src/identity.domains.auth.routes.register.rs` |
| Register route enforces proof-of-work when enabled | passed | `apps/identity-api/src/identity.domains.auth.routes.register.rs` |
| Register route applies dual rate limit | passed | `apps/identity-api/src/identity.domains.auth.routes.register.rs` |
| Register route resolves supported country to data region | passed | `apps/identity-api/src/identity.domains.auth.routes.register.rs` |
| Register route maps HTTP request to onboarding input through a tested helper | passed | `apps/identity-api/src/identity.domains.auth.routes.register.rs` |
| Register route mapping test covers data region, workspace, IP, and user-agent | passed | `apps/identity-api/src/identity.domains.auth.routes.register.tests.rs` |
| Onboarding exposes register command | passed | `apps/identity-api/src/identity.domains.auth.onboarding.rs` |
| Onboarding validates normalized email | passed | `apps/identity-api/src/identity.domains.auth.onboarding.rs` |
| Onboarding validates password policy | passed | `apps/identity-api/src/identity.domains.auth.onboarding.rs` |
| Onboarding applies account-level register rate limit | passed | `apps/identity-api/src/identity.domains.auth.onboarding.rs` |
| Onboarding creates the user account transaction | passed | `apps/identity-api/src/identity.domains.auth.onboarding.rs` |
| Onboarding records password history | passed | `apps/identity-api/src/identity.domains.auth.onboarding.rs` |
| Registration creates personal tenant | passed | `apps/identity-api/src/identity.domains.auth.db.account.rs` |
| Registration creates human principal | passed | `apps/identity-api/src/identity.domains.auth.db.account.rs` |
| Registration creates pending verification user | passed | `apps/identity-api/src/identity.domains.auth.db.account.rs` |
| Registration creates personal workspace | passed | `apps/identity-api/src/identity.domains.auth.db.account.rs` |
| Registration creates workspace policy | passed | `apps/identity-api/src/identity.domains.auth.db.account.rs` |
| Registration creates owner workspace membership | passed | `apps/identity-api/src/identity.domains.auth.db.account.rs` |
| Registration writes user.registered audit event | passed | `apps/identity-api/src/identity.domains.auth.db.account.rs` |
| Registration issues verification email token | passed | `apps/identity-api/src/identity.domains.auth.db.account.rs` |
| OpenAPI source exports register path | passed | `apps/identity-api/src/identity.http.openapi.rs` |
| Generated OpenAPI contains /auth/register | passed | `apps/identity-api/openapi.json` |
| Generated OpenAPI exposes register operation id | passed | `apps/identity-api/openapi.json` |
| Seeded OpenAPI smoke registers an account before login | passed | `scripts/test-openapi-contract.mjs` |
| Seeded OpenAPI smoke expects register success or idempotent conflict | passed | `scripts/test-openapi-contract.mjs` |

## Decision

Identity register parity evidence is covered for repository cutover gates.

## Regeneration

```bash
pnpm check:migration-identity-register
tools/migration/identity-register.mjs --write
```
