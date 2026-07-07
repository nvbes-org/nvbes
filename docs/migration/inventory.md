# Migration Inventory

## Status

Initialized. Not signed for cutover.

- `total_domain_rows`: 7
- `pending_domain_rows`: 7
- `total_source_element_rows`: 7
- `pending_source_element_rows`: 7

## Scope

This inventory tracks every source element that must receive a migration
decision before the Big Bang cutover.

The generated evidence is stored in
`docs/migration/inventory.generated.json` and refreshed with:

```bash
node tools/migration/inventory.mjs --write
pnpm check:migration-inventory
```

Inventory validation also checks generation provenance: the generated JSON must
declare `node tools/migration/inventory.mjs --write` and deterministic generation.

Secret owner and rotation evidence is stored in
`docs/migration/secret-map.generated.json` and refreshed with:

```bash
node tools/migration/secret-map.mjs --write
pnpm check:migration-secret-map
```

Before a production cutover, run the strict gate:

```bash
node tools/migration/secret-map.mjs --strict
```

Worker job replacement evidence is stored in
`docs/migration/job-map.generated.json` and refreshed with:

```bash
node tools/migration/job-map.mjs --write
pnpm check:migration-job-map
```

Before a production cutover, run the strict gate:

```bash
node tools/migration/job-map.mjs --strict
```

Bucket, queue and event-topic evidence is stored in
`docs/migration/resource-map.generated.json` and refreshed with:

```bash
node tools/migration/resource-map.mjs --write
pnpm check:migration-resource-map
```

Before a production cutover, run the strict gate:

```bash
node tools/migration/resource-map.mjs --strict
```

## Classification

| Classification | Meaning |
|---|---|
| keep | migrate as product capability |
| rebuild | replace with new implementation |
| remove | delete without migration |
| replace | move to a new provider, adapter or contract |

## Domains

| Domain | Owner | Status | Evidence |
|---|---|---|---|
| Identity | Identity owner | pending inventory | generated route/table inventory plus session, credential and MFA migration evidence |
| Workspace/Authz | Workspace/Authz owner | pending inventory | generated table inventory plus tenant, membership, role and policy migration evidence |
| Drive | Drive owner | pending inventory | generated route/table/resource inventory plus metadata, object, share and quota evidence |
| Billing/Usage | Billing/Usage owner | pending inventory | generated table/job inventory plus entitlement, ledger and webhook evidence |
| Audit/Privacy | Audit/Privacy owner | pending inventory | generated table/job inventory plus audit log, export, deletion and retention evidence |
| Developer Platform | Developer Platform owner | pending inventory | generated route/table inventory plus app, token, webhook and SDK evidence |
| Cloud/Internal | Cloud/Internal owner | pending inventory | generated infrastructure/resource inventory plus provisioning and support evidence |

## Source Elements

| Element | Path or source | Classification | Owner | Decision evidence |
|---|---|---|---|---|
| public endpoints | to inventory | pending | migration lead | docs/migration/inventory.generated.json and OpenAPI parity matrix |
| internal jobs | generated inventory | pending | migration lead | docs/migration/job-map.generated.json and worker replacement record |
| SQL tables | to inventory | pending | data lead | docs/migration/data-map.generated.json and reconciliation coverage record |
| object buckets | generated inventory | pending | data lead | docs/migration/resource-map.generated.json and retention owner record |
| queues/topics | generated inventory | pending | infra lead | docs/migration/resource-map.generated.json and replay/DLQ owner record |
| secrets | to inventory | pending | security lead | docs/migration/secret-map.generated.json and rotation owner record |
| docs | to inventory | pending | migration lead | OSS export checks and private-doc exclusion report |

## Exit Criteria

- every active feature has a target implementation;
- every table has a migration decision;
- every secret has an owner and rotation decision;
- every critical job has a replacement or removal decision;
- every blocking risk has a mitigation owner.
