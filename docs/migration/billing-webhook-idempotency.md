# Billing Webhook Idempotency Evidence

## Status

- status: passed
- checks: 15
- passed: 15
- failed: 0

## Rules

- Every evidence row must be generated from the billing webhook source contract.
- `passed` requires the configured file or OpenAPI document to contain the expected pattern.
- Summary counters must match evidence rows.
- Targeted test must name the webhook idempotency check required by parity.
- Generation provenance must identify sources, write command and targeted test.

## Evidence

| Check | Status | Path |
|---|---:|---|
| OpenAPI exposes POST /webhooks/stripe | passed | `apps/identity-api/openapi.json` |
| OpenAPI export includes Stripe webhook handler | passed | `apps/identity-api/src/identity.http.openapi.rs` |
| Axum route accepts Stripe webhooks | passed | `apps/identity-api/src/identity.domains.billing.routes.webhooks.rs` |
| Webhook route enforces rate limiting before processing | passed | `apps/identity-api/src/identity.domains.billing.routes.webhooks.rs` |
| Handler locks existing provider event rows | passed | `apps/identity-api/src/identity.domains.billing.webhooks.handlers.rs` |
| Processed webhooks are treated as duplicates | passed | `apps/identity-api/src/identity.domains.billing.webhooks.logic.rs` |
| Failed or received webhooks are replayable | passed | `apps/identity-api/src/identity.domains.billing.webhooks.logic.rs` |
| Insert path is idempotent on provider_event_id | passed | `apps/identity-api/src/identity.domains.billing.webhooks.handlers.rs` |
| Duplicate path returns duplicate without enqueueing | passed | `apps/identity-api/src/identity.domains.billing.webhooks.handlers.rs` |
| Replay path resets failed or received events | passed | `apps/identity-api/src/identity.domains.billing.webhooks.handlers.rs` |
| Queued webhook job uses provider_event_id as idempotency key | passed | `apps/identity-api/src/identity.domains.billing.jobs.rs` |
| Database constrains provider_event_id uniqueness | passed | `apps/identity-api/migrations/0001_initial_schema.sql` |
| Unit test covers failed replay classification | passed | `apps/identity-api/src/identity.domains.billing.webhooks.tests.logic.rs` |
| Unit test covers received replay classification | passed | `apps/identity-api/src/identity.domains.billing.webhooks.tests.logic.rs` |
| Unit test covers processed duplicate classification | passed | `apps/identity-api/src/identity.domains.billing.webhooks.tests.logic.rs` |

## Decision

Billing webhook idempotency and replay evidence is covered for repository cutover gates.

## Regeneration

```bash
pnpm check:migration-billing-webhook-idempotency
tools/migration/billing-webhook-idempotency.mjs --write
```
