# Identity Developer Console Design

## Goal

Build a dedicated `apps/developer-web` product for nvbes Identity developers.

The V0 must be complete enough for a tenant developer admin to manage OAuth integrations from one console: OAuth clients, marketplace approval state, consent screen configuration, scope registry, service accounts, secret rotation, webhook replay, sandbox tenant visibility, token debugging, integration logs, and health checks.

## Product Boundary

`developer-web` is a first-class app, not a route inside `identity-web`.

Product responsibilities:

- `identity-web`: personal account, login, MFA, sessions, and identity settings.
- `developer-web`: developer docs and integration operations for OAuth clients, service accounts, scopes, secrets, webhooks, logs, sandbox, and diagnostics.
- `enterprise-web`: tenant-wide administration when that product exists.
- `identity-api`: security boundary for developer permissions, OAuth data, service accounts, token diagnostics, webhook operations, and audit.

The V0 ships as a dedicated developer product because developer workflows mix public docs, tenant-scoped operational tooling, and security-sensitive integration actions. Folding this into personal account UX would blur responsibilities and create avoidable redesign pressure.

## Scope

In scope for V0:

- new `apps/developer-web` React/Vite app;
- authenticated developer console shell;
- public landing/docs shell only where needed to enter the console;
- developer RBAC with predefined roles;
- API facade under `/api/v1/developer`;
- OAuth client list/detail/create/update/revoke views;
- internal OAuth marketplace with approved, pending, rejected, and suspended states;
- per-client consent screen settings;
- central scope registry with descriptions, risk levels, owners, and lifecycle state;
- service account list/detail and OAuth client association workflows;
- guided client secret rotation with overlap window;
- webhook endpoint list, delivery history, failed delivery replay, and audit;
- sandbox tenant summary and reset/request actions when backed by API state;
- token debugger for JWT/introspection, claims, scopes, audience, expiry, and access decision;
- integration logs filtered by tenant, client, service account, event type, and severity;
- health checks for redirect URIs, JWKS, OIDC discovery, webhooks, and SCIM config;
- targeted frontend and backend tests.

Out of scope for V0:

- public third-party app store distribution;
- arbitrary custom developer role builder;
- full SCIM provisioning lifecycle management;
- tenant creation from the developer console;
- billing, pricing, or commercial upgrade flows;
- replacing existing OAuth authorization, token issuance, or service-account primitives;
- cross-tenant marketplace publishing.

## Architecture

Frontend:

- `apps/developer-web` uses React, Vite, TanStack Router, TanStack Query, `@nvbes/web-runtime`, `@nvbes/web-ui`, Zod, and lucide-react.
- The app has a dense console shell with a stable sidebar, top tenant/workspace context, and detail panes.
- API calls live in focused files: schemas, typed API functions, route loaders/hooks, and page components.
- Deterministic fixture data is allowed only for public docs copy and empty-state examples. Authenticated console data must come from API contracts.

Backend:

- `apps/identity-api/src/identity.domains.developer.*` is a facade domain.
- The facade enforces developer permissions before delegating to existing OAuth, service-account, security, federation, billing-webhook, and audit primitives.
- New persistence is limited to developer-specific product state: role assignments, scope metadata, marketplace records, consent screen config, secret rotation records, webhook endpoint/delivery metadata, sandbox bindings, health check results, and token debugger audit events.
- Existing tables remain the source of truth for OAuth clients, principals, tenants, workspaces, sessions, and issued token semantics.

## Roles And Permissions

Predefined roles:

- `developer_admin`: all developer permissions.
- `app_manager`: manage OAuth clients, consent screens, marketplace submissions, and secrets.
- `webhook_manager`: manage webhook endpoints and replay failed deliveries.
- `log_viewer`: read logs and health check results.
- `integration_tester`: use token debugger, OAuth playground, sandbox, and health checks without mutating production apps.
- `docs_viewer`: read public docs and non-sensitive catalog metadata.

Permission groups:

- Apps: read, create, update, revoke.
- Marketplace: read, submit, approve, reject, suspend.
- Scopes: read, propose, update metadata, deprecate.
- Secrets: read metadata, rotate, revoke old version.
- Service accounts: read, create, update, suspend, attach client.
- Webhooks: read, manage endpoints, replay delivery.
- Logs: read filtered integration logs.
- Tools: inspect tokens, run OAuth test flow, run health checks, manage sandbox.
- RBAC: assign and revoke developer roles.

Every backend route authorizes independently. UI visibility is convenience only.

## Domain Model

Core V0 concepts:

- `DeveloperRoleAssignment`: tenant id, principal id, role, assigned by, created at, revoked at.
- `OAuthMarketplaceApp`: tenant id, client id, status, submitted by, reviewed by, review reason, timestamps.
- `OAuthConsentScreenConfig`: client id, product name, logo URL, support URL, privacy URL, terms URL, localized description, updated by.
- `ScopeRegistryEntry`: scope key, display name, description, risk level, owner team, lifecycle state, allowed audiences.
- `ClientSecretRotation`: client id, active secret version, previous secret expiry, overlap ends at, rotated by, revoked at.
- `DeveloperWebhookEndpoint`: tenant id, name, URL, signing secret metadata, status, subscriptions.
- `DeveloperWebhookDelivery`: endpoint id, tenant id, event id, status, attempt count, response status, error message, timestamps.
- `SandboxTenantBinding`: tenant id, sandbox tenant id, status, data profile, reset requested at.
- `IntegrationHealthCheck`: tenant id, target type, target id, check kind, status, observed value summary, checked at.
- `TokenDebugSession`: tenant id, actor id, token hash prefix, result summary, access decision, created at.

No V0 table stores plaintext secrets or raw access tokens.

## API Contracts

Route prefix: `/api/v1/developer`.

Required routes:

- `GET /context`: current tenant, developer roles, permissions, available workspaces.
- `GET /overview`: summary cards for apps, secrets, webhooks, logs, sandbox, and health.
- `GET /oauth-clients`: OAuth client list with marketplace, consent, secret, and health summaries.
- `POST /oauth-clients`: create client through existing OAuth primitives.
- `GET /oauth-clients/{clientId}`: client detail.
- `PATCH /oauth-clients/{clientId}`: update safe client metadata and redirect URIs.
- `POST /oauth-clients/{clientId}/revoke`: revoke client.
- `GET /marketplace/apps`: internal marketplace catalog.
- `POST /marketplace/apps/{clientId}/submit`: submit client for approval.
- `POST /marketplace/apps/{clientId}/review`: approve, reject, or suspend.
- `GET /oauth-clients/{clientId}/consent-screen`: read consent screen config.
- `PUT /oauth-clients/{clientId}/consent-screen`: save consent screen config.
- `GET /scopes`: scope registry.
- `POST /scopes`: propose or create scope metadata.
- `PATCH /scopes/{scopeKey}`: update registry metadata.
- `GET /service-accounts`: service account summaries.
- `GET /service-accounts/{principalId}`: service account detail.
- `POST /service-accounts`: create service account.
- `POST /service-accounts/{principalId}/oauth-clients`: attach or create OAuth client.
- `POST /oauth-clients/{clientId}/secrets/rotation`: start secret rotation with overlap window.
- `POST /oauth-clients/{clientId}/secrets/{versionId}/revoke`: revoke old secret version.
- `GET /webhooks`: webhook endpoints and health summaries.
- `POST /webhooks`: create endpoint.
- `PATCH /webhooks/{endpointId}`: update endpoint.
- `GET /webhooks/{endpointId}/deliveries`: delivery history.
- `POST /webhooks/deliveries/{deliveryId}/replay`: replay failed delivery.
- `GET /logs`: filtered integration logs.
- `POST /token-debugger/inspect`: inspect token and return claims, scopes, audience, expiry, and access decision.
- `GET /sandbox`: sandbox tenant status.
- `POST /sandbox/reset`: request sandbox reset.
- `GET /health-checks`: latest health check results.
- `POST /health-checks/run`: run selected checks.

Error categories:

- unauthenticated;
- tenant context missing;
- developer role missing;
- permission denied;
- invalid client;
- invalid scope;
- unsafe redirect URI;
- secret overlap conflict;
- webhook replay not eligible;
- token expired;
- token audience mismatch;
- token signature invalid;
- health check failed;
- sandbox unavailable;
- stale version conflict.

## Frontend Information Architecture

Navigation:

- Overview
- OAuth apps
- Marketplace
- Scopes
- Service accounts
- Secrets
- Webhooks
- Logs
- Token debugger
- Sandbox
- Health checks
- Developer roles

Primary workflows:

1. Developer admin creates an OAuth client.
2. Admin configures redirect URIs, allowed scopes, audiences, and consent screen.
3. Admin submits the client to the internal marketplace.
4. Reviewer approves or rejects the app with reason.
5. Admin rotates a client secret with an explicit overlap period.
6. Integration tester inspects a token and sees claims, expiry, audience, scope coverage, and access decision.
7. Webhook manager replays a failed delivery and sees the new attempt.
8. Integration tester runs health checks and sees actionable failures.

The UI must be operational, not promotional: stable tables, compact filters, right-side detail panels, explicit empty states, and no nested cards.

## Security And Audit

Requirements:

- all authenticated console routes require tenant context;
- every mutating route writes an audit event;
- secret values are shown only once after creation or rotation;
- token debugger never stores raw token values;
- webhook replay is allowed only for failed or retryable deliveries from the same tenant;
- marketplace review actions require reviewer permission distinct from submission permission;
- sandbox actions never affect production tenant data;
- health checks redact secrets, tokens, and full webhook signatures from responses;
- logs and diagnostics are tenant-isolated and pagination-bound.

## Testing

Frontend tests:

- schema parsing for all developer API responses;
- permission display and route guard helpers;
- OAuth client list/detail states;
- scope risk display and filtering;
- secret rotation overlap validation;
- token debugger result rendering;
- webhook delivery replay eligibility;
- health check status rendering.

Backend tests:

- pure RBAC permission mapping;
- route authorization for each permission group;
- tenant isolation on list/detail endpoints;
- marketplace submit/review transitions;
- consent screen validation;
- scope registry lifecycle transitions;
- secret rotation overlap calculation;
- webhook replay eligibility and audit;
- token debugger signature/audience/scope decision cases;
- health check result normalization.

Validation after implementation:

- targeted frontend typecheck, lint, and tests for `developer-web`;
- `cargo check --workspace` after Rust changes;
- targeted Rust tests for developer domain;
- root affected checks when shared libraries or package scripts change.

## Rollout Slices

1. Scaffold `developer-web` and developer API facade with RBAC tests.
2. Add developer context, navigation, and overview.
3. Add OAuth client, marketplace, consent screen, and scope registry contracts.
4. Add service account and secret rotation workflows.
5. Add webhooks, logs, token debugger, sandbox, and health checks.
6. Add route guards, audit coverage, and error states.
7. Add targeted tests and validation.

The V0 is complete when a developer admin can create and govern an OAuth integration, rotate its secret, inspect an access token, replay a failed webhook, verify integration health, and see the relevant logs from `developer-web` without leaving the console.
