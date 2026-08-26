# nvbes Developer Experience and Portal Design

> **Status: future product surface, outside active V1.** Developer Portal work
> resumes only after selection of the first product and a separate FinOps gate.
> See the [active direction](../../product/nvbes-product-strategy.md).

Date: 2026-06-12
Status: Approved for specification review

## Goal

Build a dedicated developer experience for nvbes Account:

- A public developer site for API reference and quickstarts.
- A connected developer portal for OAuth apps, redirects, client IDs, token inspection, OAuth playground, logs, and Account webhooks.
- Clean OpenAPI output and generated SDK assets.
- Fine-grained developer RBAC with predefined roles and explicit permissions.

The V1 must be production-connected. Surfaces that create, inspect, or mutate Account configuration must call backend endpoints with real authorization checks. Documentation pages can be static content in the frontend repo.

## Product Decisions

The developer experience will be a new frontend app:

- Project: `apps/console-web`
- Package name: `nvbes-console-web`
- Nx tags: `type:app`, `domain:identity`, `layer:app`
- Runtime: React, Vite, TanStack Router, TanStack Query, TypeScript, shared `@nvbes/web-ui` and `@nvbes/web-runtime`

The portal will not be implemented as a section under `apps/account-web`. It needs a wider, workflow-oriented layout and a dedicated information architecture.

The backend remains in `apps/account-service`. A new `developer` domain will act as a product facade over OAuth, sessions, logs, webhooks, OpenAPI metadata, and SDK information. This avoids leaking internal OAuth or tenant details directly into the frontend.

## Routing

Public routes in `console-web`:

- `/`
- `/quickstarts/react`
- `/quickstarts/rust-axum`
- `/quickstarts/node`
- `/quickstarts/curl`
- `/api-reference`

Connected portal routes:

- `/portal`
- `/portal/apps`
- `/portal/apps/:clientId`
- `/portal/tokens/inspect`
- `/portal/oauth/playground`
- `/portal/logs`
- `/portal/webhooks`
- `/portal/settings/roles`

Unauthenticated users who enter `/portal/*` are redirected to Account login with a return URL back to the developer portal.

## Developer RBAC

V1 uses predefined developer roles with explicit permissions. Role names never grant behavior by themselves. Backend checks resolve role assignments into permissions and enforce the permission required by each endpoint.

Predefined roles:

- `developer_admin`
- `app_manager`
- `webhook_manager`
- `log_viewer`
- `integration_tester`
- `docs_viewer`

Permissions:

- `developer.apps.read`
- `developer.apps.create`
- `developer.apps.update_redirects`
- `developer.apps.revoke`
- `developer.webhooks.read`
- `developer.webhooks.manage`
- `developer.logs.read`
- `developer.tokens.inspect`
- `developer.oauth.playground`
- `developer.rbac.manage`
- `developer.docs.read`

Default role matrix:

| Role | Permissions |
| --- | --- |
| `developer_admin` | All developer permissions |
| `app_manager` | `developer.apps.read`, `developer.apps.create`, `developer.apps.update_redirects`, `developer.apps.revoke` |
| `webhook_manager` | `developer.webhooks.read`, `developer.webhooks.manage`, `developer.apps.read` |
| `log_viewer` | `developer.logs.read`, `developer.apps.read`, `developer.webhooks.read` |
| `integration_tester` | `developer.tokens.inspect`, `developer.oauth.playground`, `developer.apps.read` |
| `docs_viewer` | `developer.docs.read` |

Tenant `owner`, `admin`, and `security_admin` roles can bootstrap developer role assignments. Day-to-day portal actions use developer permissions.

## Backend Design

Add a new flat Rust domain under `apps/account-service/src`:

- `identity.domains.developer.mod.rs`
- `identity.domains.developer.rbac.rs`
- `identity.domains.developer.rbac.db.rs`
- `identity.domains.developer.apps.routes.rs`
- `identity.domains.developer.apps.service.rs`
- `identity.domains.developer.tokens.routes.rs`
- `identity.domains.developer.logs.routes.rs`
- `identity.domains.developer.webhooks.routes.rs`
- `identity.domains.developer.webhooks.service.rs`
- `identity.domains.developer.openapi.rs`

Routes should be nested under `/developer` in the Account service:

- `GET /developer/me`
- `GET /developer/apps`
- `POST /developer/apps`
- `GET /developer/apps/{clientId}`
- `PATCH /developer/apps/{clientId}/redirects`
- `DELETE /developer/apps/{clientId}`
- `POST /developer/tokens/inspect`
- `POST /developer/oauth/playground/exchange`
- `GET /developer/logs`
- `GET /developer/webhooks`
- `POST /developer/webhooks`
- `PATCH /developer/webhooks/{endpointId}`
- `DELETE /developer/webhooks/{endpointId}`
- `GET /developer/roles`
- `POST /developer/roles/assignments`
- `DELETE /developer/roles/assignments/{assignmentId}`

The developer domain delegates to existing OAuth services for client creation, listing, policies, and revocation. It wraps those calls with developer RBAC and response shapes that match portal workflows.

## Persistence

Add migrations for:

- `developer_role_assignments`
- `developer_role_permissions`
- `developer_webhook_endpoints`
- `developer_webhook_subscriptions`
- `developer_webhook_deliveries`

Role assignments are tenant-scoped and principal-scoped. Webhook endpoints are tenant-scoped and can subscribe to the supported Identity events:

- `user.created`
- `login.failed`
- `session.revoked`
- `client.created`

Webhook signing secrets must be recoverable by the delivery worker for HMAC signing, so they are encrypted at rest and displayed only once when created or rotated. Secret hashes can be stored alongside encrypted values for lookup or audit checks, but hashes cannot replace the encrypted signing material.

## Portal Screens

### Overview

Shows recent OAuth apps, webhook health, recent login failures, OpenAPI status, and SDK links. Empty states should guide the user to create their first app.

### Apps

Supports app creation, redirect URI configuration, client ID copy, client secret one-time display, app detail, scope and policy visibility, and revocation.

### API Reference and SDKs

Renders the Identity OpenAPI document from `/api/openapi.json`. Links to generated TypeScript and Rust SDK packages. Node and curl examples are documented as quickstarts for V1.

### Quickstarts

Provide maintained examples for:

- React
- Rust Axum
- Node
- curl

Quickstarts use real nvbes endpoint names, environment variables, and redirect URI examples.

### Token Inspector

Accepts pasted JWT or access token input. It decodes header and claims client-side for readability, then calls the backend inspector when the user has `developer.tokens.inspect`. Server inspection validates token status, tenant, client, subject, scopes, expiry, and revocation state where available.

### OAuth Playground

Builds authorization URLs, PKCE verifier/challenge, state, nonce, redirect URI, scopes, and client selection. Code exchange is performed through backend support only when the user has `developer.oauth.playground`.

### Logs

Provides filters for:

- `user_id`
- `client_id`
- `tenant_id`
- event type
- date range

V1 logs focus on developer-relevant Identity events: app creation, redirect changes, login failures, session revocations, token inspection attempts, and webhook deliveries.

### Webhooks

Supports endpoint creation, event subscriptions, secret display on create/rotate, delivery history, and delivery status. V1 event types are `user.created`, `login.failed`, `session.revoked`, and `client.created`.

## Frontend Design

The developer portal should feel like an operational console, not a marketing page. Use dense but readable layouts, a persistent sidebar for portal routes, clear toolbar actions, status chips, code blocks, and tables with filters.

The public developer site can be more editorial, but the first viewport must clearly signal "nvbes Developers" and direct users to quickstarts, API reference, and the portal. Avoid decorative-only hero sections. The first screen should help a developer start building immediately.

Use lucide icons for app, webhook, token, log, copy, external link, and warning actions. Text must fit on mobile and desktop. Cards should be reserved for repeated resources, not nested layout containers.

## OpenAPI and SDKs

The developer routes must be included in `IdentityApiDoc` with stable request and response schemas. The existing `scripts/generate-openapi.sh` remains the source of truth for exporting OpenAPI and regenerating SDK inputs.

V1 acceptance criteria:

- `/api/openapi.json` includes developer routes and schemas.
- `libs/ts/identity-sdk-core/openapi.json` is updated by generation.
- TypeScript generated types include developer endpoint shapes.
- Rust SDK generation path remains documented in `libs/ts/identity-sdk-core/package.json`.
- Node quickstarts use the TypeScript SDK package when generated client coverage exists.
- Quickstarts reference generated or stable SDK APIs where they exist.

## Security

All connected portal endpoints require authenticated Identity sessions. Mutating actions require recent step-up when consistent with existing OAuth management behavior.

Sensitive data rules:

- Client secrets and webhook secrets are one-time display only.
- Token inspector must not log raw token bodies.
- Logs API must enforce `developer.logs.read` and tenant boundaries.
- Playground exchange must enforce selected client ownership and redirect allowlists.
- Webhook signing must be mandatory for all outgoing deliveries.

## Testing and Validation

Rust validation:

- Unit tests for developer role to permission mapping.
- Endpoint tests for permission denial and allowed access.
- OpenAPI contract tests asserting developer routes exist.
- Webhook event tests for payload shape and delivery records.

Frontend validation:

- `pnpm exec nx run console-web:typecheck`
- `pnpm exec nx run console-web:lint`
- Targeted tests for permission gating, client ID copy, token inspector parsing, and webhook form validation.
- Browser verification for desktop and mobile layouts.

Repository validation:

- Run `cargo check --workspace` after Rust changes.
- Run targeted frontend checks for `console-web`.
- Run `pnpm generate:openapi` after OpenAPI changes.

## Implementation Boundaries

In scope for V1:

- New `console-web` app.
- Developer RBAC predefined roles and explicit permissions.
- Developer backend facade in `account-service`.
- OAuth app dashboard with redirects and client ID copy.
- OpenAPI and SDK generation updates.
- Quickstarts for React, Rust Axum, Node, and curl.
- Token inspector.
- OAuth playground.
- Logs filtered by user, client, and tenant.
- Account webhooks for `user.created`, `login.failed`, `session.revoked`, and `client.created`.

Out of scope for V1:

- Tenant-customizable roles.
- Full ABAC policy builder.
- Separate `developer-api` service.
- Public marketplace or app directory.
- Multi-environment app promotion workflows.
