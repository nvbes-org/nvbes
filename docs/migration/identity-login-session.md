# Identity Login Session Evidence

## Status

- status: passed
- checks: 27
- passed: 27
- failed: 0

## Rules

- Every evidence row must be generated from the identity login/session source contract.
- `passed` requires the configured source, OpenAPI document, or migration risk source to contain the expected pattern.
- Summary counters must match evidence rows.
- Targeted tests must name refresh, logout, and seeded auth smoke checks required by parity.
- Generation provenance must identify sources, write command and targeted tests.

## Evidence

| Check | Status | Path |
|---|---:|---|
| Login identifier route is present | passed | `apps/identity-api/src/identity.domains.auth.routes.login.identifier.rs` |
| Password login route is present | passed | `apps/identity-api/src/identity.domains.auth.routes.login.pwd.rs` |
| Primary login applies throttle rules | passed | `apps/identity-api/src/identity.domains.auth.sessions.create.verify.rs` |
| Primary login verifies stored password | passed | `apps/identity-api/src/identity.domains.auth.sessions.create.verify.rs` |
| Failed login records risk event | passed | `apps/identity-api/src/identity.domains.auth.sessions.create.verify.rs` |
| Failed login records audit event | passed | `apps/identity-api/src/identity.domains.auth.sessions.create.verify.rs` |
| Successful login issues a session-bound token | passed | `apps/identity-api/src/identity.domains.auth.sessions.create.session.rs` |
| Successful login stores cached session | passed | `apps/identity-api/src/identity.domains.auth.sessions.create.session.rs` |
| Successful login records risk event | passed | `apps/identity-api/src/identity.domains.auth.sessions.create.session.rs` |
| Successful login records audit event | passed | `apps/identity-api/src/identity.domains.auth.sessions.create.session.rs` |
| OpenAPI contains logout route | passed | `apps/identity-api/openapi.json` |
| OpenAPI contains session listing route | passed | `apps/identity-api/openapi.json` |
| Logout revocation state is isolated from audit persistence | passed | `apps/identity-api/src/identity.domains.auth.sessions.mgmt.rs` |
| Logout revokes refresh tokens for the current session | passed | `apps/identity-api/src/identity.domains.auth.sessions.mgmt.rs` |
| Logout deletes the current session cache | passed | `apps/identity-api/src/identity.domains.auth.sessions.mgmt.rs` |
| Logout records session revoked audit event | passed | `apps/identity-api/src/identity.domains.auth.sessions.mgmt.rs` |
| Logout state test proves only current session and refresh token are revoked | passed | `apps/identity-api/src/identity.domains.auth.sessions.mgmt.tests.rs` |
| Refresh middleware retries expired access tokens | passed | `apps/identity-api/src/identity.http.middleware.jwt.session_refresh.rs` |
| Refresh rejects revoked or expired cached sessions | passed | `apps/identity-api/src/identity.http.middleware.jwt.session_refresh.rs` |
| Refresh rotates cached access-token hash | passed | `apps/identity-api/src/identity.http.middleware.jwt.session_refresh.rs` |
| Refresh cookie construction is pure-testable | passed | `apps/identity-api/src/identity.http.middleware.jwt.session_refresh.rs` |
| Refresh cookie tests cover authuser-scoped secure cookies | passed | `apps/identity-api/src/identity.http.middleware.jwt.session_refresh.rs` |
| Seeded OpenAPI smoke calls login identifier challenge | passed | `scripts/test-openapi-contract.mjs` |
| Seeded OpenAPI smoke calls password challenge | passed | `scripts/test-openapi-contract.mjs` |
| Seeded OpenAPI smoke requires a session cookie | passed | `scripts/test-openapi-contract.mjs` |
| Seeded OpenAPI smoke lists sessions after login | passed | `scripts/test-openapi-contract.mjs` |
| Session migration risk has explicit clearing/login return decision | passed | `tools/migration/risk-register.evidence.mjs` |

## Decision

Identity login/logout/refresh parity evidence is covered for repository cutover gates.

## Regeneration

```bash
pnpm check:migration-identity-login-session
tools/migration/identity-login-session.mjs --write
```
