# ADR 0002 - Reconstruction Plateforme Big Bang Zero Dette

## Status

Accepted for planning. Production execution remains gated by the migration
runbook.

## Context

The current repository contains product code, cloud-specific adapters, internal
operational material, OSS export material and legacy provider integrations in a
single private workspace. The target architecture requires clear product,
cloud, internal and migration boundaries before a Big Bang migration can be
executed without temporary debt.

## Decision

Rebuild nvbes in parallel around explicit domains, stable contracts and strict
OSS/Cloud/Internal boundaries, then cut over in one controlled migration window.

The source private repository remains the source of truth. The public OSS
repository is generated from an allowlist and must never include private
blueprints, migration runbooks, internal docs, Cloud-only adapters, secrets or
deployment material.

## Consequences

- Product logic must live outside application bootstraps.
- Provider-specific integrations must be isolated behind ports and adapters.
- Public APIs require OpenAPI before they are exposed.
- Critical events require versioned schemas, outbox publication and replay
  semantics.
- Migration data requires export, transform, import, reject logs and
  reconciliation evidence.
- Cutover cannot proceed while a runtime legacy dependency is still required.

## Validation

The decision is enforced by:

- `pnpm check:structure`;
- `pnpm check:contracts`;
- `pnpm check:product-boundaries`;
- `pnpm check:migration-inventory`;
- `pnpm check:migration-data-map`;
- `pnpm check:migration-secret-map`;
- `pnpm check:migration-job-map`;
- `pnpm check:migration-resource-map`;
- `pnpm check:migration-risk-register`;
- `pnpm check:migration-gate-evidence`;
- `pnpm check:oss-boundaries`;
- `pnpm check:migration-artifacts`;
- `pnpm check:secrets`;
- the gate criteria in `docs/blueprint/nvbes-full-restructure-big-bang-zero-debt.plan.md`;
- the operational sequence in `docs/migration/nvbes-big-bang-migration-runbook.md`.
