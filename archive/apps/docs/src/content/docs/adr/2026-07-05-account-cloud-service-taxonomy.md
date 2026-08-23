---
title: "ADR 2026-07-05 - Account Cloud Service Taxonomy"
description: Architecture Decision Record - nvbes platform
---

## Status

Accepted for the Account/Cloud Big Bang refactor.

## Context

The repository previously used runtime names that described old product slices
instead of durable ownership boundaries. The target platform needs bounded
contexts that describe ownership rather than historical implementation names.

The Big Bang cutover must remove legacy names from runtime code, packages,
environment variables, deployment artifacts, observability, and public docs.
Migration evidence may reference old names only while proving the cutover.

## Decision

Adopt the following runtime taxonomy. The exact historical rename map is
maintained as migration evidence in
`docs/migration/account-cloud-big-bang.inventory.md`; this ADR records the
post-cutover names that are allowed to remain in non-migration docs and runtime
surfaces.

| Target runtime | Owner |
|---|---|
| `billing-service` | Billing |
| `cloud-service` | Cloud |
| `cloud-web` | Cloud |
| `cloud-worker` | Cloud |
| `console-web` | Developer |
| `account-service` | Account |
| `account-web` | Account |
| `account-worker` | Account |
| `backoffice-service` | Backoffice |
| `backoffice-web` | Backoffice |
| `gateway-cloud` | Gateway Cloud |

Create these service runtimes during extraction:

- `developer-service`;
- `enterprise-service`.

Historical runtime aliases listed in the migration inventory are forbidden
after cutover and may remain only in approved migration evidence.

`gateway-cloud` is the only runtime name for the Cloud gateway/BFF. If GraphQL
remains its public protocol, GraphQL is a protocol choice, not a service name.

## Ownership

Account owns authentication, sessions, MFA, WebAuthn, OAuth/OIDC protocol
execution, token exchange, JWT/JWKS, users, principals, account settings,
account export/delete, account chooser, step-up decisions, and account-facing
billing orchestration.

Account does not own billing source of truth. Account-facing billing management
is a facade over Billing gRPC contracts for plan summary, portal entry,
invoice summary, and payment-method entry points.

Cloud owns workspace lifecycle, workspace membership source of truth, Cloud
resource policies, Drive files/folders/uploads/downloads/share links/trash,
quotas, storage metadata, scanning orchestration, public Cloud APIs, and Cloud
worker jobs.

Account may keep authorization read models and projections needed for auth
decisions, but Account must not write Cloud workspace tables after cutover.

Billing owns catalog, prices, plans, subscriptions, entitlements, usage ledger,
checkout, portal sessions, invoices, payment methods, dunning, PSP adapters,
webhook persistence, reconciliation, and Billing worker jobs.

Developer owns developer apps, OAuth client registration metadata, scope
catalog, marketplace metadata, developer tokens, signed webhooks, webhook
delivery settings, sandbox, API logs, and the Console product API.

Enterprise owns SSO/SAML/SCIM governance, enterprise policies, access reviews,
break-glass, admin elevation, trust/security posture, compliance workflows,
and policy decisions consumed by Account and Cloud.

Backoffice owns internal operations, support tooling, fraud/risk operations,
and internal-only admin workflows. `scope:internal` code must not be imported by
OSS or Cloud projects.

Gateway Cloud owns public Cloud client composition, authentication at the edge,
coarse routing, typed gRPC clients, response shaping, deadlines, correlation
IDs, and BFF-specific caching. It does not own business state.

## Integration Rules

- Cross-context mutations use gRPC commands, versioned events, or explicit
  outbox workflows.
- Services must not depend on other service app crates or packages.
- Services must not directly query another service database.
- Product crates must not import infrastructure adapters directly.
- Public APIs are generated from contracts; SDK clients consume those
  contracts instead of hand-maintained fetch calls.

## Consequences

- Mechanical renames must update Cargo package/bin names, Nx project names,
  package names, scripts, deploy manifests, images, environment variables,
  metrics labels, dashboards, and docs.
- Temporary rename helpers are allowed only inside migration scripts and must be
  deleted before the final old-name gate.
- Existing docs that describe legacy names become migration evidence or must be
  rewritten to the target taxonomy.
- Account, Cloud, Developer, Enterprise, and Billing require explicit product
  boundaries before production cutover.

## Validation

Cutover is blocked unless:

- forbidden aliases are absent outside approved migration evidence;
- `cargo check --workspace` passes;
- `pnpm verify` passes;
- Account no longer writes Cloud workspace source-of-truth tables;
- Billing remains source of truth for billing data;
- Developer and Enterprise behavior no longer lives inside Account;
- Gateway Cloud has no business persistence.
