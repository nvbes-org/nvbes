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

| Check | Status | Path |
|---|---:|---|
| OpenAPI exposes developer webhook listing | passed | `apps/identity-api/openapi.json` |
| OpenAPI exposes developer webhook creation | passed | `apps/identity-api/openapi.json` |
| OpenAPI exposes developer webhook revocation | passed | `apps/identity-api/openapi.json` |
| OpenAPI export includes developer webhook handlers | passed | `apps/identity-api/src/identity.http.openapi.rs` |
| Webhook creation returns signing secret once | passed | `apps/identity-api/src/identity.domains.developer.webhooks.routes.rs` |
| Webhook mutations require manage permission | passed | `apps/identity-api/src/identity.domains.developer.webhooks.routes.rs` |
| Developer console exposes delivery replay route | passed | `apps/identity-api/src/identity.domains.developer.routes.rs` |
| Webhook signatures use HMAC-SHA256 | passed | `apps/identity-api/src/identity.domains.developer.webhooks.delivery.rs` |
| Signature header includes timestamp and v1 digest | passed | `apps/identity-api/src/identity.domains.developer.webhooks.delivery.rs` |
| Signature binds event id into signed payload | passed | `apps/identity-api/src/identity.domains.developer.webhooks.delivery.rs` |
| Signature binds raw payload bytes | passed | `apps/identity-api/src/identity.domains.developer.webhooks.delivery.rs` |
| Signature freshness window rejects replay outside tolerance | passed | `apps/identity-api/src/identity.domains.developer.webhooks.delivery.rs` |
| Replay classifier only allows failed or pending deliveries | passed | `apps/identity-api/src/identity.domains.developer.webhooks.delivery.rs` |
| Unit test proves signature binding | passed | `apps/identity-api/src/identity.domains.developer.webhooks.delivery.rs` |
| Unit test proves replay window rejection | passed | `apps/identity-api/src/identity.domains.developer.webhooks.delivery.rs` |
| Unit test proves replay idempotency guard | passed | `apps/identity-api/src/identity.domains.developer.webhooks.delivery.rs` |
| Replay route applies replayability guard | passed | `apps/identity-api/src/identity.domains.developer.routes.webhooks.rs` |
| Replay insert is idempotent for the original delivery | passed | `apps/identity-api/src/identity.domains.developer.routes.webhooks.rs` |
| Portal schema prevents duplicate replays per original delivery | passed | `apps/identity-api/migrations/0011_developer_portal.sql` |
| Console schema prevents duplicate replays per original delivery | passed | `apps/identity-api/migrations/0008_developer_console.sql` |
| Backend route test covers replay response shape | passed | `apps/identity-api/src/identity.domains.developer.tests.webhooks.rs` |
| Developer web lists console webhook endpoints | passed | `apps/developer-web/src/developer.api.ts` |
| Developer web lists delivery attempts | passed | `apps/developer-web/src/developer.api.ts` |
| Developer web calls replay endpoint | passed | `apps/developer-web/src/developer.api.ts` |
| Developer web validates webhook delivery status | passed | `apps/developer-web/src/developer.schemas.ts` |
| Developer web gates replay actions by delivery status | passed | `apps/developer-web/src/pages/WebhooksPage.helpers.ts` |
| Developer web test blocks delivered delivery replay | passed | `apps/developer-web/src/__tests__/developer.webhooks.test.ts` |

## Decision

Developer signed webhook and replay-idempotency evidence is covered for repository cutover gates.

## Regeneration

```bash
pnpm check:migration-developer-signed-webhooks
tools/migration/developer-signed-webhooks.mjs --write
```
