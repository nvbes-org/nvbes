# Billing Multi-PSP E2E Evidence

## Status

- status: passed
- checks: 27
- passed: 27
- failed: 0

## Rules

- PSP webhook intake must live in billing-service, not Account.
- PSP webhook processing, analytics, emails and dunning must live in Billing worker.
- The repository smoke proof must be deterministic and must not call external PSP APIs.
- Multi-PSP continuity must cover Stripe, Mollie, CB, primary ownership and fallback eligibility.

## Evidence

| Check | Status | Path |
|---|---:|---|
| billing-service exposes Stripe webhook intake | passed | `apps/billing-service/src/billing.domains.webhooks.rs` |
| billing-service exposes Mollie webhook intake | passed | `apps/billing-service/src/billing.domains.webhooks.rs` |
| Stripe intake verifies raw webhook signature | passed | `libs/rust/billing/src/stripe_webhook_intake.rs` |
| Stripe intake enqueues asynchronous processing | passed | `libs/rust/billing/src/stripe_webhook_intake.rs` |
| Mollie classic webhook intake parses id-only callback | passed | `libs/rust/billing/src/mollie.webhooks.rs` |
| Mollie intake enqueues asynchronous processing | passed | `apps/billing-service/src/billing.domains.webhooks.rs` |
| Stripe webhook queue name is provider-scoped | passed | `libs/rust/billing/src/jobs.rs` |
| Mollie webhook queue name is provider-scoped | passed | `libs/rust/billing/src/jobs.rs` |
| Billing worker dispatches Stripe webhook jobs | passed | `apps/billing-worker/src/billing.worker.jobs.rs` |
| Billing worker dispatches Mollie webhook jobs | passed | `apps/billing-worker/src/billing.worker.jobs.rs` |
| Billing worker dispatches billing email submission jobs | passed | `apps/billing-worker/src/billing.worker.jobs.rs` |
| Stripe worker processes Billing webhook events | passed | `apps/billing-worker/src/billing.worker.jobs.stripe.rs` |
| Stripe worker captures Billing analytics | passed | `apps/billing-worker/src/billing.worker.jobs.stripe.rs` |
| Stripe worker enqueues Billing email | passed | `apps/billing-worker/src/billing.worker.jobs.stripe.rs` |
| Mollie worker processes provider payment updates | passed | `apps/billing-worker/src/billing.worker.jobs.mollie.rs` |
| Mollie worker finalizes initial subscriptions | passed | `apps/billing-worker/src/billing.worker.jobs.mollie.rs` |
| Mollie worker enqueues Billing email | passed | `apps/billing-worker/src/billing.worker.jobs.mollie.rs` |
| Billing email uses the Billing integration queue | passed | `apps/billing-worker/src/billing.worker.email.rs` |
| Billing worker owns Billing webhook analytics | passed | `apps/billing-worker/src/billing.worker.analytics.rs` |
| Billing worker publishes workspace update events after PSP webhooks | passed | `apps/billing-worker/src/billing.worker.workspace_updates.rs` |
| Billing worker publishes workspace plan update events after PSP webhooks | passed | `apps/billing-worker/src/billing.worker.workspace_updates.rs` |
| Billing worker tests plan update payload stability | passed | `apps/billing-worker/src/billing.worker.workspace_updates.rs` |
| Billing worker owns dunning processing loop | passed | `apps/billing-worker/src/billing.worker.loop.rs` |
| Workspace effects apply only to primary provider subscriptions | passed | `libs/rust/billing/src/stripe_webhook_workspace_effects.rs` |
| Workspace effects update dunning state from webhook outcomes | passed | `libs/rust/billing/src/stripe_webhook_workspace_effects.rs` |
| Multi-PSP continuity evidence is generated and passed | passed | `docs/migration/billing-multi-psp-continuity.generated.json` |
| Account Service and worker do not own PSP webhook runtime | passed | `apps/identity-service/src + apps/account-worker/src` |

## Decision

Billing multi-PSP repository E2E evidence is covered without external PSP dependencies. Production cutover still requires live environment evidence.

## Verification

```bash
pnpm check:migration-billing-multi-psp-e2e
```

## Regeneration

```bash
node tools/migration/billing-multi-psp-e2e.mjs --write
```
