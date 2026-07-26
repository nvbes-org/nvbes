# Developer Signed Webhook Evidence

## Status

- status: passed
- checks: 27
- passed: 27
- failed: 0

## Rules

- Every evidence row must be generated from the developer signed webhook source contract.
- `passed` requires the configured file or OpenAPI document to contain the expected pattern.
- Summary counters must match evidence rows.
- Targeted tests must name the backend and frontend webhook checks required by parity.
- Generation provenance must identify sources, write command and targeted tests.

## Evidence

| Check                                                              | Status | Path                                                                   |
| ------------------------------------------------------------------ | -----: | ---------------------------------------------------------------------- |
| OpenAPI exposes developer webhook listing                          | passed | `apps/developer-service/openapi.json`                                  |
| OpenAPI exposes developer webhook creation                         | passed | `apps/developer-service/openapi.json`                                  |
| OpenAPI exposes developer webhook revocation                       | passed | `apps/developer-service/openapi.json`                                  |
| OpenAPI export includes developer webhook handlers                 | passed | `apps/developer-service/src/developer.http.openapi.rs`                 |
| Webhook creation returns signing secret once                       | passed | `apps/developer-service/src/developer.http.routes.webhooks.rs`         |
| Webhook mutations require manage permission                        | passed | `apps/developer-service/src/developer.http.routes.webhooks.rs`         |
| Developer console exposes delivery replay route                    | passed | `apps/developer-service/src/developer.http.routes.rs`                  |
| Webhook signatures use HMAC-SHA256                                 | passed | `apps/developer-service/src/developer.webhooks.signature.rs`           |
| Signature header includes timestamp and v1 digest                  | passed | `apps/developer-service/src/developer.webhooks.signature.rs`           |
| Signature binds event id into signed payload                       | passed | `apps/developer-service/src/developer.webhooks.signature.rs`           |
| Signature binds raw payload bytes                                  | passed | `apps/developer-service/src/developer.webhooks.signature.rs`           |
| Signature freshness window rejects replay outside tolerance        | passed | `apps/developer-service/src/developer.webhooks.signature.rs`           |
| Replay classifier only allows failed or pending deliveries         | passed | `apps/developer-service/src/developer.webhooks.signature.rs`           |
| Unit test proves signature binding                                 | passed | `apps/developer-service/src/developer.webhooks.signature.rs`           |
| Unit test proves replay window rejection                           | passed | `apps/developer-service/src/developer.webhooks.signature.rs`           |
| Unit test proves replay idempotency guard                          | passed | `apps/developer-service/src/developer.webhooks.signature.rs`           |
| Replay service applies replayability guard                         | passed | `apps/developer-service/src/developer.grpc.webhooks.rs`                |
| Replay insert is idempotent for the original delivery              | passed | `apps/developer-service/src/developer.grpc.webhooks.rs`                |
| Portal schema prevents duplicate replays per original delivery     | passed | `apps/account-service/migrations/0011_developer_portal.sql`            |
| Console schema prevents duplicate replays per original delivery    | passed | `apps/account-service/migrations/0008_developer_console.sql`           |
| Backend contract test covers replay idempotency and response shape | passed | `apps/developer-service/src/developer.grpc.webhooks.contract_tests.rs` |
| Console web lists console webhook endpoints                        | passed | `apps/console-web/src/developer.api.ts`                                |
| Console web lists delivery attempts                                | passed | `apps/console-web/src/developer.api.ts`                                |
| Console web calls replay endpoint                                  | passed | `apps/console-web/src/developer.api.ts`                                |
| Console web validates webhook delivery status                      | passed | `apps/console-web/src/developer.schemas.ts`                            |
| Console web gates replay actions by delivery status                | passed | `apps/console-web/src/pages/WebhooksPage.helpers.ts`                   |
| Console web test blocks delivered delivery replay                  | passed | `apps/console-web/src/__tests__/developer.webhooks.test.ts`            |

## Decision

Developer signed webhook and replay-idempotency evidence is covered for repository cutover gates.

## Regeneration

```bash
pnpm check:migration-developer-signed-webhooks
node tools/migration/developer-signed-webhooks.mjs --write
```
