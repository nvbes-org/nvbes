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

| Check                                                                      | Status | Path                                                                                |
| -------------------------------------------------------------------------- | -----: | ----------------------------------------------------------------------------------- |
| Identity OpenAPI exposes OAuth client listing                              | passed | `apps/account-service/openapi.json`                                                 |
| Identity OpenAPI exposes OAuth client creation                             | passed | `apps/account-service/openapi.json`                                                 |
| Identity OpenAPI exposes OAuth client revocation                           | passed | `apps/account-service/openapi.json`                                                 |
| Identity OpenAPI exposes OAuth client policy creation                      | passed | `apps/account-service/openapi.json`                                                 |
| Developer OpenAPI exposes token inspection                                 | passed | `apps/developer-service/openapi.json`                                               |
| Identity OpenAPI export includes OAuth client handlers                     | passed | `apps/account-service/src/identity.http.openapi.rs`                                 |
| Developer OpenAPI export includes token inspection handler                 | passed | `apps/developer-service/src/developer.http.openapi.rs`                              |
| OAuth routes wire list, create, and revoke                                 | passed | `apps/account-service/src/identity.domains.oauth.routes.clients.rs`                 |
| OAuth management mutations require recent step-up                          | passed | `apps/account-service/src/identity.domains.oauth.routes.clients.rs`                 |
| OAuth creation returns generated client secret once                        | passed | `apps/account-service/src/identity.domains.oauth.clients.create.rs`                 |
| OAuth revocation invalidates client refresh tokens                         | passed | `apps/account-service/src/identity.domains.oauth.clients.revoke.rs`                 |
| OAuth revocation invalidates pushed authorization requests                 | passed | `apps/account-service/src/identity.domains.oauth.clients.revoke.rs`                 |
| OAuth revocation invalidates authorization codes                           | passed | `apps/account-service/src/identity.domains.oauth.clients.revoke.rs`                 |
| OAuth revocation invalidates device codes                                  | passed | `apps/account-service/src/identity.domains.oauth.clients.revoke.rs`                 |
| Machine token test records last-used and audit metadata                    | passed | `apps/account-service/src/identity.domains.oauth.flows.client_credentials.tests.rs` |
| Developer token inspection requires permission                             | passed | `apps/developer-service/src/developer.http.routes.tools.rs`                         |
| Developer console token debug records hash prefix only                     | passed | `apps/developer-service/src/developer.http.routes.tools.rs`                         |
| Backend token debug unit test covers tenant mismatch precedence            | passed | `apps/developer-service/src/developer.http.routes.tools.rs`                         |
| Developer console lists OAuth clients with consent and health status       | passed | `apps/developer-service/src/developer.http.routes.oauth.rs`                         |
| Generated SDK core includes OAuth clients path                             | passed | `libs/ts/identity-sdk-core/src/types.gen.ts`                                        |
| Generated SDK core includes OAuth client result types                      | passed | `libs/ts/identity-sdk-core/src/types.gen.ts`                                        |
| Developer service owns token inspection result types                       | passed | `apps/developer-service/src/developer.http.types.rs`                                |
| Handwritten identity client lists OAuth clients                            | passed | `libs/ts/identity-client/src/index.ts`                                              |
| Handwritten identity client revokes OAuth clients                          | passed | `libs/ts/identity-client/src/index.ts`                                              |
| Console web consumes OAuth client list API                                 | passed | `apps/console-web/src/developer.api.ts`                                             |
| Console web consumes token debug API                                       | passed | `apps/console-web/src/developer.api.ts`                                             |
| Console web validates OAuth client summaries                               | passed | `apps/console-web/src/developer.schemas.ts`                                         |
| Console web validates debug token responses                                | passed | `apps/console-web/src/developer.schemas.ts`                                         |
| Console web schema test parses OAuth client summaries                      | passed | `apps/console-web/src/__tests__/developer.schemas.test.ts`                          |
| Console web token test rejects raw token material in parsed debug response | passed | `apps/console-web/src/__tests__/developer.token-health.test.ts`                     |

## Decision

Developer OAuth app and token inspection evidence is covered for repository cutover gates.

## Regeneration

```bash
pnpm check:migration-developer-oauth-tokens
node tools/migration/developer-oauth-tokens.mjs --write
```
