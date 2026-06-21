# Developer OAuth and Token Evidence

## Status

- status: passed
- checks: 30
- passed: 30
- failed: 0

## Rules

- Every evidence row must be generated from the developer OAuth/token source contract.
- `passed` requires the configured file or OpenAPI document to contain the expected pattern.
- Summary counters must match evidence rows.
- Targeted tests must name the backend and frontend token checks required by parity.
- Generation provenance must identify sources, write command and targeted tests.

## Evidence

| Check | Status | Path |
|---|---:|---|
| OpenAPI exposes OAuth client listing | passed | `apps/identity-api/openapi.json` |
| OpenAPI exposes OAuth client creation | passed | `apps/identity-api/openapi.json` |
| OpenAPI exposes OAuth client revocation | passed | `apps/identity-api/openapi.json` |
| OpenAPI exposes OAuth client policy creation | passed | `apps/identity-api/openapi.json` |
| OpenAPI exposes developer token inspection | passed | `apps/identity-api/openapi.json` |
| OpenAPI export includes OAuth client handlers | passed | `apps/identity-api/src/identity.http.openapi.rs` |
| OpenAPI export includes token inspection handler | passed | `apps/identity-api/src/identity.http.openapi.rs` |
| OAuth routes wire list, create, and revoke | passed | `apps/identity-api/src/identity.domains.oauth.routes.clients.rs` |
| OAuth management mutations require recent step-up | passed | `apps/identity-api/src/identity.domains.oauth.routes.clients.rs` |
| OAuth creation returns generated client secret once | passed | `apps/identity-api/src/identity.domains.oauth.clients.create.rs` |
| OAuth revocation invalidates client refresh tokens | passed | `apps/identity-api/src/identity.domains.oauth.clients.revoke.rs` |
| OAuth revocation invalidates pushed authorization requests | passed | `apps/identity-api/src/identity.domains.oauth.clients.revoke.rs` |
| OAuth revocation invalidates authorization codes | passed | `apps/identity-api/src/identity.domains.oauth.clients.revoke.rs` |
| OAuth revocation invalidates device codes | passed | `apps/identity-api/src/identity.domains.oauth.clients.revoke.rs` |
| Machine token test records last-used and audit metadata | passed | `apps/identity-api/src/identity.domains.oauth.flows.client_credentials.tests.rs` |
| Developer token inspection requires permission | passed | `apps/identity-api/src/identity.domains.developer.tokens.routes.rs` |
| Developer console token debug records hash prefix only | passed | `apps/identity-api/src/identity.domains.developer.routes.tokens.rs` |
| Backend token debug unit tests cover allowed, expired, and tenant mismatch | passed | `apps/identity-api/src/identity.domains.developer.routes.tokens.tests.rs` |
| Developer console lists OAuth clients with consent and health status | passed | `apps/identity-api/src/identity.domains.developer.routes.oauth.clients.rs` |
| Generated SDK core includes OAuth clients path | passed | `libs/ts/identity-sdk-core/src/types.gen.ts` |
| Generated SDK core includes OAuth client result types | passed | `libs/ts/identity-sdk-core/src/types.gen.ts` |
| Generated SDK core includes token inspection result types | passed | `libs/ts/identity-sdk-core/src/types.gen.ts` |
| Handwritten identity client lists OAuth clients | passed | `libs/ts/identity-client/src/index.ts` |
| Handwritten identity client revokes OAuth clients | passed | `libs/ts/identity-client/src/index.ts` |
| Developer web consumes OAuth client list API | passed | `apps/developer-web/src/developer.api.ts` |
| Developer web consumes token debug API | passed | `apps/developer-web/src/developer.api.ts` |
| Developer web validates OAuth client summaries | passed | `apps/developer-web/src/developer.schemas.ts` |
| Developer web validates debug token responses | passed | `apps/developer-web/src/developer.schemas.ts` |
| Developer web schema test parses OAuth client summaries | passed | `apps/developer-web/src/__tests__/developer.schemas.test.ts` |
| Developer web token test rejects raw token material in parsed debug response | passed | `apps/developer-web/src/__tests__/developer.token-health.test.ts` |

## Decision

Developer OAuth app and token inspection evidence is covered for repository cutover gates.

## Regeneration

```bash
pnpm check:migration-developer-oauth-tokens
tools/migration/developer-oauth-tokens.mjs --write
```
