# Account Cloud Big Bang Inventory

Date: 2026-07-05

This inventory freezes the runtime taxonomy before the Account/Cloud Big Bang
cutover. It is the only approved long-lived document that may keep legacy
runtime names during the rename and boundary extraction work.

## Command Evidence

All commands were run from the repository root on branch
`codex/account-cloud-big-bang-refactor`.

```bash
rtk rg -n "billing-api|drive-api|developer-web|identity-api|identity-web|drive-web|internal-admin|internal-admin-web|gateway-graphql|identity-worker|drive-worker" .
rtk rg --files apps libs contracts docs scripts tools deploy infrastructure | sort
rtk pnpm exec nx show projects --json
rtk cargo metadata --format-version 1 --no-deps
```

Observed baseline:

- old runtime name references: 3,793 lines;
- inventory file list: 2,709 files;
- Nx projects: 35;
- Cargo workspace packages: 25.

`pnpm exec nx show projects --json` emitted an engine warning because the repo
requires Node `>=25.0.0` while the local runtime is Node `v24.14.0`; Nx still
returned the project list. `cargo metadata` initially exposed a workspace
manifest hygiene issue: `apps/billing-worker` inherited `hex` without a root
workspace dependency. The root workspace now declares `hex = "0.4"` so metadata
can be generated.

## Frozen Runtime Name Map

| Legacy runtime | Target runtime | Owner |
|---|---|---|
| `apps/billing-api` / `nvbes-billing-api` | `apps/billing-service` / `nvbes-billing-service` | Billing |
| `apps/drive-api` / `nvbes-drive-api` | `apps/cloud-service` / `nvbes-cloud-service` | Cloud |
| `apps/developer-web` / `nvbes-developer-web` | `apps/console-web` / `nvbes-console-web` | Developer |
| `apps/identity-api` / `nvbes-identity-api` | `apps/account-service` / `nvbes-account-service` | Account |
| `apps/identity-web` / `nvbes-identity-web` | `apps/account-web` / `nvbes-account-web` | Account |
| `apps/internal-admin` / `nvbes-internal-admin` | `apps/backoffice-service` / `nvbes-backoffice-service` | Backoffice |
| `apps/internal-admin-web` / `nvbes-internal-admin-web` | `apps/backoffice-web` / `nvbes-backoffice-web` | Backoffice |
| `apps/gateway-graphql` / `nvbes-gateway-graphql` | `apps/gateway-cloud` / `nvbes-gateway-cloud` | Gateway Cloud |
| `apps/identity-worker` / `nvbes-identity-worker` | `apps/account-worker` / `nvbes-account-worker` | Account |
| `apps/drive-worker` / `nvbes-drive-worker` | `apps/cloud-worker` / `nvbes-cloud-worker` | Cloud |

New services required by the cutover:

- `apps/developer-service` / `nvbes-developer-service`;
- `apps/enterprise-service` / `nvbes-enterprise-service`.

## Current Project Inventory

Nx projects:

```text
analytics-posthog
audit
billing
billing-api
billing-client
billing-worker
core
developer-web
drive-api
drive-web
drive-worker
email-scaleway
email-templates
enterprise-web
go-workspace
http-client
identity-api
identity-client
identity-sdk
identity-sdk-backend
identity-sdk-core
identity-sdk-web
identity-web
identity-worker
internal-admin
internal-admin-sdk-core
internal-admin-web
observability
platform
ports
python-workspace
rust-workspace
tenancy
web-runtime
web-ui
```

Cargo workspace packages:

```text
nvbes-analytics-posthog
nvbes-audit
nvbes-billing
nvbes-billing-api
nvbes-billing-worker
nvbes-core
nvbes-dpop
nvbes-drive-api
nvbes-drive-worker
nvbes-email
nvbes-email-scaleway
nvbes-gateway-graphql
nvbes-identity-api
nvbes-identity-sdk
nvbes-identity-worker
nvbes-internal-admin
nvbes-observability
nvbes-platform
nvbes-ports
nvbes-product-analytics
nvbes-redis
nvbes-region
nvbes-scan
nvbes-storage
nvbes-tenancy
```

## Data Sources And Ownership

| Current source | Current owner | Target owner | Cutover decision |
|---|---|---|---|
| `apps/identity-api/migrations` | Identity | Account, Developer, Enterprise, Cloud projections | Split by bounded context; Account keeps auth/session/user data only. |
| `apps/drive-api/migrations` | Drive | Cloud | Cloud owns workspace, files, folders, uploads, sharing, quotas, privacy exports, and object metadata. |
| `apps/billing-api/migrations` | Billing | Billing | Billing remains source of truth for plans, subscriptions, entitlements, usage, invoices, PSP webhooks, dunning, and reconciliation. |
| `apps/internal-admin` SQL reads | Internal Admin | Backoffice | Backoffice must consume internal contracts/read models instead of direct product persistence. |
| `libs/rust/billing/src` | Billing domain library | Billing product library | Keep as Billing product crate unless later ADR renames it to `libs/rust/products/billing`. |
| `libs/rust/storage`, `libs/rust/scan` | Shared primitives | Cloud through ports | Cloud composes storage/scan ports; product logic must not depend on adapters directly. |

Known environment data handles:

- `NVBES_DATABASE_URL`: default Account/legacy Identity database handle;
- `NVBES_DRIVE_DATABASE_URL`: legacy Drive database handle, target Cloud;
- `NVBES_BILLING_DATABASE_URL`: Billing database handle;
- staging aliases under `NVBES_STAGING_*` that must be renamed or removed at cutover.

## Contracts

Current contracts are concentrated in:

- `contracts/openapi/manifest.json`;
- `contracts/protobuf/nvbes/billing/v1/billing.proto`;
- `contracts/events/manifest.json`;
- event schemas for `identity.user.created`, `workspace.membership.created`,
  `drive.file.created`, and Billing lifecycle events;
- GraphQL schema under `contracts/graphql` for the current gateway surface.

Target contracts must split into Account, Cloud, Billing, Developer,
Enterprise, Backoffice-internal, and Gateway Cloud contracts. Cross-context
mutations must use gRPC commands, versioned events, or outbox workflows.

## Jobs And Workers

Current worker runtimes:

- `identity-worker`: Identity background jobs and legacy worker alias scripts;
- `drive-worker`: Drive maintenance, privacy export, and queue processing jobs;
- `billing-worker`: PSP webhook processing, billing email, dunning, analytics,
  and workspace billing update publishing.

Target worker ownership:

- Account worker: auth/account/session/privacy jobs owned by Account;
- Cloud worker: Cloud workspace, Drive, storage, scan, export, and cleanup jobs;
- Billing worker: PSP, invoice, dunning, entitlements, usage, and reconciliation
  jobs;
- Developer and Enterprise workers should exist only if their bounded contexts
  need async ownership after extraction.

## Runtime Configuration

Ports and URLs currently exposed in `.env.example` and scripts:

| Legacy variable | Current value or role | Target |
|---|---|---|
| `NVBES_IDENTITY_API_BASE_URL` | `http://localhost:4000` | `NVBES_ACCOUNT_SERVICE_BASE_URL` |
| `VITE_IDENTITY_API_BASE_URL` | browser Identity API base URL | `VITE_ACCOUNT_SERVICE_BASE_URL` or generated SDK config |
| `NVBES_DRIVE_API_BASE_URL` | `http://localhost:4002` | `NVBES_CLOUD_SERVICE_BASE_URL` |
| `VITE_DRIVE_API_BASE_URL` | browser Drive API base URL | `VITE_CLOUD_SERVICE_BASE_URL` or generated SDK config |
| `NVBES_BILLING_API_PORT` | `4020` | `NVBES_BILLING_SERVICE_PORT` |
| `NVBES_GATEWAY_GRAPHQL_PORT` | `4030` | `NVBES_GATEWAY_CLOUD_PORT` |
| `NVBES_DRIVE_WORKER_METRICS_BIND_ADDR` | `127.0.0.1:4101` | `NVBES_CLOUD_WORKER_METRICS_BIND_ADDR` |
| `NVBES_IDENTITY_WORKER_METRICS_BIND_ADDR` | `127.0.0.1:4102` | `NVBES_ACCOUNT_WORKER_METRICS_BIND_ADDR` |
| `NVBES_BILLING_WORKER_METRICS_BIND_ADDR` | `127.0.0.1:4104` | Keep; Billing worker remains Billing. |

Script entry points currently include `dev-identity-api.sh`, `dev-drive-api.sh`,
`dev-billing-api.sh`, `dev-gateway-graphql.sh`, `dev-identity-worker.sh`,
`dev-drive-worker.sh`, `dev-developer-web.sh`, `dev-identity-web.sh`, and
`dev-drive-web.sh`. Each must be renamed or deleted before the old-name gate.

## Images, Deployments, And Observability

Current image and service names appear in:

- `deploy/oss/compose/compose.yaml`;
- `deploy/oss/helm/nvbes/values.yaml`;
- `infrastructure/local/observability/prometheus*.yml`;
- `infrastructure/local/observability/grafana-dashboards/*.json`;
- `infrastructure/local/observability/generate-grafana-dashboards.mjs`;
- `infrastructure/environments/production/alloy.config.alloy`;
- `infrastructure/environments/production/observability.env.example`;
- `infrastructure/environments/production/docker-compose.observability.yml`.

Current observability labels include `identity-api`, `drive-api`,
`billing-api`, `identity-worker`, `drive-worker`, `internal-admin`, and
`identity-web`. Target labels must use only `account-service`,
`cloud-service`, `billing-service`, `gateway-cloud`, `account-worker`,
`cloud-worker`, `billing-worker`, `backoffice-service`, and target web app
names.

## Cutover Constraints

- No legacy runtime name may remain outside this inventory and explicit
  migration evidence after the deletion gate.
- No service may read or write another service database directly.
- Account may keep authorization read models/projections, but Cloud owns
  workspace source of truth.
- Account-facing billing screens call Account, and Account delegates to Billing;
  Billing remains source of truth.
- Developer and Enterprise product behavior must leave Account before cutover.
