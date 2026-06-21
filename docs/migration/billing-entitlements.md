# Billing Entitlements Evidence

## Status

- status: passed
- checks: 35
- passed: 35
- failed: 0

## Rules

- Every evidence row must be generated from the billing entitlements source contract.
- `passed` requires the configured file, contract, or generated migration document to contain the expected pattern.
- Summary counters must match evidence rows.
- Targeted tests must name entitlement and reconciliation checks required by parity.
- Generation provenance must identify sources, write command and targeted tests.

## Evidence

| Check | Status | Path |
|---|---:|---|
| Shared billing type exposes product entitlements | passed | `libs/rust/billing/src/types.rs` |
| Entitlements include upload permission | passed | `libs/rust/billing/src/types.rs` |
| Entitlements include share-link permission | passed | `libs/rust/billing/src/types.rs` |
| Entitlements include storage allowance | passed | `libs/rust/billing/src/types.rs` |
| Entitlements include API key limits | passed | `libs/rust/billing/src/types.rs` |
| Entitlements include billing lock state | passed | `libs/rust/billing/src/types.rs` |
| Shared entitlement view derives product permissions | passed | `libs/rust/billing/src/views.rs` |
| Shared entitlement view locks degraded statuses | passed | `libs/rust/billing/src/views.rs` |
| Shared entitlement view maps plan to API key limit | passed | `libs/rust/billing/src/views.rs` |
| Active entitlement behavior is covered by unit test | passed | `libs/rust/billing/src/views.rs` |
| Degraded subscription lock behavior is covered by unit test | passed | `libs/rust/billing/src/views.rs` |
| Invoice overage calculation is covered by unit test | passed | `libs/rust/billing/src/views.rs` |
| Invoice estimate charges storage and seat overages | passed | `libs/rust/billing/src/views.rs` |
| Billing policy helper locks degraded subscription statuses | passed | `libs/rust/billing/src/shared.rs` |
| Identity billing overview returns entitlements | passed | `apps/identity-api/src/identity.domains.billing.service.overview.rs` |
| Identity usage response exposes billable storage and seats | passed | `apps/identity-api/src/identity.domains.billing.service.overview.rs` |
| Identity exposes billing entitlements endpoint | passed | `apps/identity-api/src/identity.domains.billing.routes.manage.rs` |
| Identity entitlement endpoint requires billing authorization | passed | `apps/identity-api/src/identity.domains.billing.routes.manage.rs` |
| Identity service returns workspace entitlements | passed | `apps/identity-api/src/identity.domains.billing.entitlements.rs` |
| Identity service re-exports shared entitlement type | passed | `apps/identity-api/src/identity.domains.billing.service.types.rs` |
| Identity billing operations enforce lock policy | passed | `apps/identity-api/src/identity.domains.billing.policy.rs` |
| Drive billing overview returns shared entitlements | passed | `apps/drive-api/src/drive.domains.billing.manage.core.rs` |
| Drive usage response exposes billable storage and seats | passed | `apps/drive-api/src/drive.domains.billing.manage.core.rs` |
| Drive persists invoice estimates for reconciliation | passed | `apps/drive-api/src/drive.domains.billing.manage.core.rs` |
| Drive invoice estimate persistence stores amount and period | passed | `apps/drive-api/src/drive.domains.billing.entitlements.rs` |
| Drive service re-exports shared billing response types | passed | `apps/drive-api/src/drive.domains.billing.types.rs` |
| Entitlement changed event schema is versioned | passed | `contracts/events/billing.entitlement.changed.v1.schema.json` |
| Entitlement event payload requires workspace and status | passed | `contracts/events/billing.entitlement.changed.v1.schema.json` |
| Event manifest includes billing entitlement changes | passed | `contracts/events/manifest.json` |
| Billing financial data requires ledger balance reconciliation | passed | `docs/migration/data-map.md` |
| Subscriptions have explicit rebuild decision | passed | `docs/migration/data-map.md` |
| Usage events have explicit rebuild decision | passed | `docs/migration/data-map.md` |
| Billing webhook processing job is mapped | passed | `docs/migration/job-map.md` |
| Billing entitlement event topic is mapped | passed | `docs/migration/resource-map.md` |
| Billing divergence risk requires ledger reconciliation | passed | `docs/migration/risk-register.generated.json` |

## Decision

Billing entitlements parity evidence is covered for repository cutover gates. Production cutover still requires accepted ledger reconciliation.

## Regeneration

```bash
pnpm check:migration-billing-entitlements
tools/migration/billing-entitlements.mjs --write
```
