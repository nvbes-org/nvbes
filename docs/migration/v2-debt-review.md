# V2 Debt Review

## Status

- decision: no-go
- reviewed_scope: repository migration docs, blueprint docs, ADRs and cutover manifests
- v1_required_findings: 2
- resolved_findings: 1
- accepted_findings: 0
- owner_acceptance: missing

## Scan Inputs

```bash
rg -n "compatibility|temporary|debt review|V2 backlog clean|V2 debt|open backlog|product owner acceptance" docs/migration docs/blueprint docs/adr -g '*.md'
pnpm check:migration-v2-debt -- --strict
pnpm check:migration-decommission -- --strict
```

## Findings

| Finding | Source | V1 impact | Required closure |
|---|---|---|---|
| Final decommission requires a clean V2 backlog decision. | `docs/migration/decommission-manifest.md` | Production completion remains blocked until the V2 backlog clean row has a debt review report, backlog query result and product owner acceptance record. | Product leads must attach acceptance evidence and mark the row `go`. |
| The V2 debt register still has one V1-required row open. | `docs/migration/v2-debt-register.md` | Strict V2 debt validation must fail until the row is closed with concrete evidence. | Close the row only after owner acceptance proves no V1-required debt remains. |

## Resolved Findings

| Finding | Source | Resolution evidence |
|---|---|---|
| Billing compatibility views were still described in a historical blueprint. | `docs/blueprint/nvbes-internal-billing-platform.plan.md` | The plan now defers runtime boundaries to `docs/adr/2026-07-05-account-cloud-service-taxonomy.md`, moves the canonical billing schema to `apps/billing-service/migrations/`, and requires compatibility helpers to be removed before the final gate. |

## Decision

`no-go`: the stale blueprint conflict is resolved, but there is still no product
owner acceptance evidence proving that no V1-required debt remains for the first
production cutover.
