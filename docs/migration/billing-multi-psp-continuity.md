# Billing Multi-PSP Continuity Evidence

## Status

- status: passed
- checks: 20
- passed: 20
- failed: 0

## Rules

- Billing provider contracts must expose Stripe, Mollie and CB consistently.
- CB Safe'R, Updat'R and Fast'R must stay behind an acquirer/PAT port, not a product runtime dependency.
- Provider subscriptions must support exactly one primary provider and fallback-eligible non-primary providers.
- Webhook side effects must mutate workspace state only for the primary provider subscription.
- Targeted tests must cover provider code parsing and multi-PSP continuity invariants.

## Evidence

| Check | Status | Path |
|---|---:|---|
| Billing provider model exposes Stripe, Mollie and CB | passed | `libs/rust/billing/src/provider.rs` |
| Provider code tests cover CB | passed | `libs/rust/billing/src/provider_tests.rs` |
| CB adapter models Safe'R, Updat'R and Fast'R services | passed | `libs/rust/billing/src/cb.rs` |
| CB Safe'R excludes recurring payments | passed | `libs/rust/billing/src/cb.rs` |
| CB Updat'R requires stored credential context | passed | `libs/rust/billing/src/cb.rs` |
| CB Fast'R requires customer initiated ecommerce context | passed | `libs/rust/billing/src/cb.rs` |
| CB integration remains behind acquirer or PAT port | passed | `libs/rust/billing/src/cb.rs` |
| Provider customer lookup does not reuse unrelated provider IDs | passed | `libs/rust/billing/src/provider_tests.rs` |
| Provider subscriptions store primary provider ownership | passed | `apps/billing-api/migrations/0002_billing_platform_core.sql` |
| Provider subscriptions store fallback eligibility | passed | `apps/billing-api/migrations/0002_billing_platform_core.sql` |
| Database enforces one primary provider subscription | passed | `apps/billing-api/migrations/0004_billing_provider_subscription_primary_fallback.sql` |
| New primary provider subscription demotes previous providers | passed | `libs/rust/billing/src/db.provider_subscriptions.rs` |
| Active non-primary provider subscriptions remain fallback eligible | passed | `libs/rust/billing/src/db.provider_subscriptions.rs` |
| Primary provider subscriptions are not fallback candidates | passed | `libs/rust/billing/src/db.provider_subscriptions.rs` |
| Inactive provider subscriptions are not fallback candidates | passed | `libs/rust/billing/src/db.provider_subscriptions.rs` |
| Webhook workspace effects apply only to primary provider subscription | passed | `libs/rust/billing/src/stripe_webhook_workspace_effects.rs` |
| Portal subscriptions expose provider-neutral primary and fallback state | passed | `libs/rust/billing/src/portal_views.subscriptions.rs` |
| Billing database enum supports CB provider | passed | `apps/billing-api/migrations/0009_billing_provider_cb.sql` |
| Billing TypeScript client supports CB provider | passed | `libs/ts/billing-client/src/billing.provider.ts` |
| GraphQL Billing provider enum supports CB | passed | `contracts/graphql/schema.graphql` |

## Decision

Billing multi-PSP continuity evidence is covered for repository cutover gates. Production cutover still requires provider sandbox E2E evidence.

## Regeneration

```bash
pnpm check:migration-billing-multi-psp-continuity
tools/migration/billing-multi-psp-continuity.mjs --write
```
