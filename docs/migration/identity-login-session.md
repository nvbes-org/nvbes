# Identity Login Session Evidence

## Status

- status: passed
- checks: 23
- passed: 23
- failed: 0

## Rules

- Every evidence row must be generated from the identity login/session source contract.
- `passed` requires the configured source, OpenAPI document, or migration risk source to contain the expected pattern.
- Summary counters must match evidence rows.
- Targeted tests must name session authentication, logout, and API contract checks required by parity.
- Generation provenance must identify sources, write command and targeted tests.

## Evidence

| Check | Status | Path |
|---|---:|---|
| Login identifier route is present | passed | `apps/identity-service/src/identity.domains.auth.routes.login.identifier.rs` |
| Password login route is present | passed | `apps/identity-service/src/identity.domains.auth.routes.login.pwd.rs` |
| Primary login applies throttle rules | passed | `apps/identity-service/src/identity.domains.auth.sessions.create.verify.rs` |
| Primary login verifies stored password and upgrades weak hashes | passed | `apps/identity-service/src/identity.domains.auth.sessions.create.verify.rs` |
| Failed login records risk event | passed | `apps/identity-service/src/identity.domains.auth.sessions.create.verify.rs` |
| Failed login records audit event | passed | `apps/identity-service/src/identity.domains.auth.sessions.create.verify.rs` |
| Successful login issues an opaque session-bound browser token | passed | `apps/identity-service/src/identity.domains.auth.sessions.create.session.rs` |
| Successful login stores cached session | passed | `apps/identity-service/src/identity.domains.auth.sessions.create.session.rs` |
| Successful login records risk event | passed | `apps/identity-service/src/identity.domains.auth.sessions.create.session.rs` |
| Successful login records audit event | passed | `apps/identity-service/src/identity.domains.auth.sessions.create.session.rs` |
| OpenAPI contains logout route | passed | `apps/identity-service/openapi.json` |
| OpenAPI contains session listing route | passed | `apps/identity-service/openapi.json` |
| Logout revocation state is isolated from audit persistence | passed | `apps/identity-service/src/identity.domains.auth.sessions.mgmt.rs` |
| Logout revokes refresh tokens for the current session | passed | `apps/identity-service/src/identity.domains.auth.sessions.mgmt.rs` |
| Logout deletes the current session cache | passed | `apps/identity-service/src/identity.domains.auth.sessions.mgmt.rs` |
| Logout records session revoked audit event | passed | `apps/identity-service/src/identity.domains.auth.sessions.mgmt.rs` |
| Logout state test proves only current session and refresh token are revoked | passed | `apps/identity-service/src/identity.domains.auth.sessions.mgmt.tests.rs` |
| Session middleware authenticates OAuth bearer requests | passed | `apps/identity-service/src/identity.http.middleware.jwt.session_refresh.rs` |
| Session middleware authenticates authuser-scoped browser cookies | passed | `apps/identity-service/src/identity.http.middleware.jwt.session_refresh.rs` |
| Browser cookies are validated against central session state | passed | `apps/identity-service/src/identity.http.middleware.jwt.session_refresh.rs` |
| Terminal authentication failures require explicit reauthentication | passed | `apps/identity-service/src/identity.http.middleware.jwt.session_refresh.rs` |
| Session recovery semantics are covered by a unit test | passed | `apps/identity-service/src/identity.http.middleware.jwt.session_refresh.rs` |
| Session migration risk has explicit clearing/login return decision | passed | `tools/migration/risk-register.evidence.mjs` |

## Decision

Identity login/logout/session authentication parity evidence is covered for repository cutover gates.

## Regeneration

```bash
pnpm check:migration-identity-login-session
node tools/migration/identity-login-session.mjs --write
```
