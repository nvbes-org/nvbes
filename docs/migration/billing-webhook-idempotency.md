# Billing Webhook Idempotency Evidence

## Status

- status: passed
- checks: 46
- passed: 46
- failed: 0

## Rules

- Every evidence row must be generated from the billing webhook source contract.
- `passed` requires the configured source to satisfy the expected presence or absence pattern.
- Summary counters must match evidence rows.
- Targeted test must name the webhook idempotency check required by parity.
- Generation provenance must identify sources, write command and targeted test.

## Evidence

| Check | Status | Path |
|---|---:|---|
| Identity OpenAPI no longer exposes PSP webhook routes | passed | `apps/identity-api/openapi.json` |
| Identity router no longer merges billing routes | passed | `apps/identity-api/src/identity.domains.mod.rs` |
| Billing API accepts Stripe webhooks | passed | `apps/billing-api/src/billing.domains.webhooks.rs` |
| Billing API accepts Mollie webhooks | passed | `apps/billing-api/src/billing.domains.webhooks.rs` |
| Stripe webhook intake is rate limited in billing-api | passed | `apps/billing-api/src/billing.domains.webhooks.rs` |
| Mollie webhook intake is rate limited in billing-api | passed | `apps/billing-api/src/billing.domains.webhooks.rs` |
| Stripe intake verifies signatures before enqueueing | passed | `libs/rust/billing/src/stripe_webhook_intake.rs` |
| Stripe intake locks existing provider event rows | passed | `libs/rust/billing/src/stripe_webhook_intake.rs` |
| Processed Stripe webhooks are treated as duplicates | passed | `libs/rust/billing/src/stripe_webhook_intake.rs` |
| Failed or received Stripe webhooks are replayable | passed | `libs/rust/billing/src/stripe_webhook_intake.rs` |
| Unit test covers processed duplicate classification | passed | `libs/rust/billing/src/stripe_webhook_intake.rs` |
| Unit test covers failed replay classification | passed | `libs/rust/billing/src/stripe_webhook_intake.rs` |
| Unit test covers received replay classification | passed | `libs/rust/billing/src/stripe_webhook_intake.rs` |
| Stripe replay path resets failed or received events | passed | `libs/rust/billing/src/stripe_webhook_intake.rs` |
| Stripe intake enqueues async processing | passed | `libs/rust/billing/src/stripe_webhook_intake.rs` |
| Stripe webhook processing has a dedicated queue | passed | `libs/rust/billing/src/jobs.rs` |
| Mollie webhook processing has a dedicated queue | passed | `libs/rust/billing/src/jobs.rs` |
| Billing email delivery has a dedicated queue | passed | `libs/rust/billing/src/jobs.rs` |
| Queued webhook jobs use provider_event_id as idempotency key | passed | `libs/rust/billing/src/jobs.rs` |
| Billing worker claims Stripe, Mollie, and billing email queues | passed | `apps/billing-worker/src/billing.worker.jobs.rs` |
| Billing worker dispatches Stripe webhook jobs | passed | `apps/billing-worker/src/billing.worker.jobs.rs` |
| Billing worker dispatches Mollie webhook jobs | passed | `apps/billing-worker/src/billing.worker.jobs.rs` |
| Billing worker dispatches billing email jobs | passed | `apps/billing-worker/src/billing.worker.jobs.rs` |
| Stripe worker calls billing-domain processing | passed | `apps/billing-worker/src/billing.worker.jobs.stripe.rs` |
| Stripe worker publishes Billing workspace updates | passed | `apps/billing-worker/src/billing.worker.jobs.stripe.rs` |
| Stripe worker enqueues billing emails after processing | passed | `apps/billing-worker/src/billing.worker.jobs.stripe.rs` |
| Billing email enqueue uses the Billing queue | passed | `apps/billing-worker/src/billing.worker.email.rs` |
| Billing email enqueue does not use Identity email.send queue | passed | `apps/billing-worker/src/billing.worker.email.rs` |
| Billing worker sends billing emails locally | passed | `apps/billing-worker/src/billing.worker.email.delivery.rs` |
| Billing email delivery is tagged as Billing domain | passed | `apps/billing-worker/src/billing.worker.email.delivery.rs` |
| Mollie worker calls billing-domain processing | passed | `apps/billing-worker/src/billing.worker.jobs.mollie.rs` |
| Mollie worker publishes Billing workspace updates | passed | `apps/billing-worker/src/billing.worker.jobs.mollie.rs` |
| Mollie worker enqueues billing emails after processing | passed | `apps/billing-worker/src/billing.worker.jobs.mollie.rs` |
| Mollie worker records failed provider events | passed | `apps/billing-worker/src/billing.worker.jobs.mollie.rs` |
| Mollie worker records processed provider events | passed | `apps/billing-worker/src/billing.worker.jobs.mollie.rs` |
| Billing worker publishes workspace:updated pubsub events | passed | `apps/billing-worker/src/billing.worker.workspace_updates.rs` |
| Billing worker publishes workspace:plan_updated pubsub events | passed | `apps/billing-worker/src/billing.worker.workspace_updates.rs` |
| Billing email enqueue accepts provider-neutral payment events | passed | `apps/billing-worker/src/billing.worker.email.rs` |
| Billing domain opens dunning cases after provider payment failures | passed | `libs/rust/billing/src/dunning_db.rs` |
| Billing domain closes dunning cases after provider payment success | passed | `libs/rust/billing/src/dunning_db.rs` |
| Billing worker schedules dunning processing | passed | `apps/billing-worker/src/billing.worker.loop.rs` |
| Billing worker records dunning processing metrics | passed | `apps/billing-worker/src/billing.worker.loop.rs` |
| Dunning processing claims due attempts safely | passed | `libs/rust/billing/src/dunning_jobs.rs` |
| Dunning processing validates batch size | passed | `libs/rust/billing/src/dunning_jobs.rs` |
| Provider event intake is idempotent per provider event | passed | `libs/rust/billing/src/db.provider_events.rs` |
| Database constrains provider event uniqueness per provider | passed | `apps/billing-api/migrations/0002_billing_platform_core.sql` |

## Decision

Billing webhook idempotency and replay evidence is covered for repository cutover gates.

## Regeneration

```bash
pnpm check:migration-billing-webhook-idempotency
tools/migration/billing-webhook-idempotency.mjs --write
```
