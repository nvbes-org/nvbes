# Internal Admin

Dedicated private back-office API.

This app owns operator-only endpoints and must stay isolated from public product APIs such as
`identity-api` and `drive-api`.

## Boundary

- Internal-only app, not exported to `nvbes-oss`.
- Back-office routes are mounted here, never in customer-facing services.
- HTTP access is protected by the shared internal token guard.
- Audited mutations require `x-nvbes-actor-principal-id` so back-office actions remain attributable.

## Billing

- `GET /admin/access-center`
- `GET /admin/command-center`
- `GET /admin/communications-center`
- `GET /admin/compliance-center`
- `GET /admin/customer-center`
- `GET /admin/developer-center`
- `GET /admin/identity-governance-center`
- `GET /admin/operations-center`
- `GET /admin/revenue-center`
- `GET /admin/region-center`
- `GET /admin/security-center`
- `GET /admin/search`
- `GET /admin/tenants/{tenantId}`
- `GET /admin/users/{principalId}`
- `GET /admin/workspaces/{workspaceId}`
- `GET /workspaces/{workspaceId}/admin/audit-events`
- `GET /workspaces/{workspaceId}/billing/admin/overview`
- `GET /workspaces/{workspaceId}/billing/admin/provider-events/failures`
- `GET /workspaces/{workspaceId}/billing/admin/search`
- `POST /workspaces/{workspaceId}/billing/admin/credit-notes`
- `POST /workspaces/{workspaceId}/billing/admin/write-offs`
- `POST /workspaces/{workspaceId}/billing/admin/refund-intents`
- `POST /workspaces/{workspaceId}/billing/admin/provider-events/replay`
- `POST /workspaces/{workspaceId}/billing/admin/provider-migrations`
- `POST /workspaces/{workspaceId}/billing/admin/grace-overrides`
- `POST /workspaces/{workspaceId}/billing/admin/manual-compensations`
- `POST /workspaces/{workspaceId}/billing/admin/exports/{exportType}`
- `GET /admin/billing/runbooks`
