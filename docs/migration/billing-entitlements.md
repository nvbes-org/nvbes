# Billing Entitlements Evidence

## Status

- status: passed
- checks: 36
- passed: 36
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
| Billing service overview returns entitlements | passed | `libs/rust/billing/src/workspace_views.rs` |
| Billing service usage response exposes billable storage and seats | passed | `libs/rust/billing/src/workspace_views.rs` |
| billing-service exposes public entitlements endpoint | passed | `apps/billing-service/src/billing.domains.public_workspace.rs` |
| Billing entitlements endpoint requires Billing read authorization | passed | `apps/billing-service/src/billing.domains.public_workspace.rs` |
| Billing service returns workspace entitlements | passed | `libs/rust/billing/src/workspace_views.rs` |
| billing-service returns shared entitlement response type | passed | `apps/billing-service/src/billing.domains.public_workspace.rs` |
| billing-service exposes internal entitlements endpoint | passed | `apps/billing-service/src/billing.domains.workspace.rs` |
| billing-service authorizes requests through Account introspection | passed | `apps/billing-service/src/billing.auth.rs` |
| Billing checkout enforces lock policy | passed | `libs/rust/billing/src/checkout_sessions.rs` |
| Billing portal operations enforce lock policy | passed | `libs/rust/billing/src/portal_actions.rs` |
| Drive projects Billing entitlement events locally | passed | `apps/cloud-service/src/drive.domains.billing.entitlements.rs` |
| Drive stores only entitlement projections from Billing | passed | `apps/cloud-service/src/drive.domains.billing.entitlements.rs` |
| Drive no longer stores Billing invoice estimates | passed | `apps/cloud-service/src/drive.domains.billing.entitlements.rs` |
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
node tools/migration/billing-entitlements.mjs --write
```
