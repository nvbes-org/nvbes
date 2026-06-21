# Identity MFA WebAuthn Evidence

## Status

- status: passed
- checks: 37
- passed: 37
- failed: 0

## Rules

- Every evidence row must be generated from the identity MFA/WebAuthn source contract.
- `passed` requires the configured source, OpenAPI document, smoke test, or migration map to contain the expected pattern.
- Summary counters must match evidence rows.
- Targeted tests must name MFA, WebAuthn option shaping, login-method ordering, and seeded auth smoke checks required by parity.
- Generation provenance must identify sources, write command and targeted tests.

## Evidence

| Check | Status | Path |
|---|---:|---|
| MFA router exposes factor list and enrollment routes | passed | `apps/identity-api/src/identity.domains.auth.routes.mfa.rs` |
| TOTP setup route is protected by JWT and step-up | passed | `apps/identity-api/src/identity.domains.auth.routes.mfa.totp.rs` |
| TOTP confirm route activates a factor | passed | `apps/identity-api/src/identity.domains.auth.routes.mfa.totp.rs` |
| TOTP enrollment generates a secret through core MFA primitives | passed | `apps/identity-api/src/identity.domains.auth.mfa.totp.rs` |
| TOTP verification rejects replayed counters | passed | `apps/identity-api/src/identity.domains.auth.mfa.totp.rs` |
| Recovery code generation stores hashed codes | passed | `apps/identity-api/src/identity.domains.auth.mfa.recovery.rs` |
| Recovery verification consumes one matching code | passed | `apps/identity-api/src/identity.domains.auth.mfa.recovery.rs` |
| Login method ordering is covered by a unit test | passed | `apps/identity-api/src/identity.domains.auth.mfa.rs` |
| MFA login challenge route is exposed | passed | `apps/identity-api/src/identity.domains.auth.routes.login.mfa.rs` |
| MFA login challenge is rate limited by IP and state | passed | `apps/identity-api/src/identity.domains.auth.routes.login.mfa.rs` |
| MFA login accepts exactly one factor | passed | `apps/identity-api/src/identity.domains.auth.routes.login.mfa_flow.rs` |
| MFA login rejects multiple factors in a deterministic unit test | passed | `apps/identity-api/src/identity.domains.auth.routes.login.mfa_flow.rs` |
| MFA failures record risk events | passed | `apps/identity-api/src/identity.domains.auth.routes.login.mfa.rs` |
| MFA failures record audit events | passed | `apps/identity-api/src/identity.domains.auth.routes.login.mfa.rs` |
| Successful MFA creates an AAL2 session | passed | `apps/identity-api/src/identity.domains.auth.routes.login.mfa.rs` |
| Passwordless WebAuthn start route is exposed | passed | `apps/identity-api/src/identity.domains.auth.routes.login.webauthn.rs` |
| Discoverable WebAuthn start route is exposed | passed | `apps/identity-api/src/identity.domains.auth.routes.login.webauthn.rs` |
| Discoverable WebAuthn finish route creates an AAL2 session | passed | `apps/identity-api/src/identity.domains.auth.routes.login.webauthn.rs` |
| WebAuthn enrollment start route is exposed | passed | `apps/identity-api/src/identity.domains.auth.routes.mfa.webauthn.rs` |
| WebAuthn enrollment requires recent AAL2 step-up | passed | `apps/identity-api/src/identity.domains.auth.routes.mfa.webauthn.rs` |
| WebAuthn enrollment finish route is exposed | passed | `apps/identity-api/src/identity.domains.auth.routes.mfa.webauthn.rs` |
| WebAuthn registration challenge state is cached | passed | `apps/identity-api/src/identity.domains.auth.webauthn.registration.start.rs` |
| WebAuthn registration start is audited | passed | `apps/identity-api/src/identity.domains.auth.webauthn.registration.start.rs` |
| WebAuthn registration finish persists passkey data | passed | `apps/identity-api/src/identity.domains.auth.webauthn.registration.finish.rs` |
| WebAuthn registration finish is audited | passed | `apps/identity-api/src/identity.domains.auth.webauthn.registration.finish.rs` |
| WebAuthn registration distinguishes passkeys from security keys | passed | `apps/identity-api/src/identity.domains.auth.webauthn.registration.options.rs` |
| WebAuthn option shaping is covered by unit tests | passed | `apps/identity-api/src/identity.domains.auth.webauthn.registration.tests.rs` |
| WebAuthn login stores challenge state | passed | `apps/identity-api/src/identity.domains.auth.webauthn.login.rs` |
| WebAuthn login success is audited | passed | `apps/identity-api/src/identity.domains.auth.webauthn.login.rs` |
| Discoverable WebAuthn login success is audited | passed | `apps/identity-api/src/identity.domains.auth.webauthn.login.discoverable.rs` |
| WebAuthn step-up challenge is supported | passed | `apps/identity-api/src/identity.domains.auth.webauthn.authentication.rs` |
| OpenAPI source includes MFA and WebAuthn routes | passed | `apps/identity-api/src/identity.http.openapi.rs` |
| Generated OpenAPI includes MFA challenge | passed | `apps/identity-api/openapi.json` |
| Generated OpenAPI includes WebAuthn enrollment | passed | `apps/identity-api/openapi.json` |
| Seeded OpenAPI smoke observes mfa_enabled in login profile | passed | `scripts/test-openapi-contract.mjs` |
| Identity MFA credentials have an explicit rebuild migration decision | passed | `docs/migration/data-map.md` |
| Legacy Drive MFA credentials have an explicit rebuild migration decision | passed | `docs/migration/data-map.md` |

## Decision

Identity MFA/WebAuthn parity evidence is covered for repository cutover gates. Credential migration remains a rebuild decision documented in the data map.

## Regeneration

```bash
pnpm check:migration-identity-mfa-webauthn
tools/migration/identity-mfa-webauthn.mjs --write
```
