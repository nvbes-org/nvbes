# Enterprise Web Tenant Admin Console Design

## Goal

Build a dedicated `apps/enterprise-web` site for customer organizations to manage the whole enterprise account.

The core product distinction is mandatory:

- `Enterprise / Tenant` is the root administration scope.
- `Workspace` is a managed resource inside the tenant.
- `account-web` stays focused on personal account, login, MFA, sessions, and identity settings.
- `cloud-web` stays focused on Drive product usage.
- `enterprise-web` owns users, workspaces, developers, policies, security, billing, usage, audit, and tenant settings.

The first complete V0 workflow is `Users`: invite people, assign tenant roles, grant module access, assign workspaces, suspend or reactivate memberships, and show policy warnings before risky changes.

## Scope

In scope:

- new frontend app boundary: `apps/enterprise-web`;
- tenant-scoped app shell, route tree, and context loading;
- left navigation for enterprise modules;
- complete `Users` V0 route;
- route shells for all other enterprise modules with real summaries and empty states;
- tenant-first API contracts for overview, users, invitations, workspace access, policies, security, billing, usage, and audit;
- precise TypeScript types and schemas, with no `any`;
- targeted frontend and backend/API tests.

Out of scope for this V0:

- custom role builder;
- SCIM, directory sync, and SAML management;
- full compliance export workflows;
- fine-grained ACLs below workspace and module scope;
- white-label theming beyond tenant identity display;
- replacing existing `account-web` or `cloud-web` flows.

## Architecture

`enterprise-web` is a first-class app, not a route inside `account-web`.

Product responsibilities:

- `account-web`: personal identity and authentication UX.
- `enterprise-web`: tenant administration UX.
- `cloud-web`: Drive product UX.
- `account-service`: security boundary for authn, authz, tenant, workspace, billing, developer, and policy data.

Frontend foundations should match the current web stack: React, Vite, TanStack Router, TanStack Query, `@nvbes/web-runtime`, existing UI patterns, and typed Account service clients.

Every `enterprise-web` route starts from tenant context. Workspace context is optional and only appears when editing workspace-scoped data.

## Navigation

Navigation:

- Overview
- Users
- Workspaces
- Developers
- Policies
- Security
- Audit logs
- Billing
- Usage
- Settings

Grouping:

- Operate: Overview, Users, Workspaces, Developers
- Govern: Policies, Security, Audit logs
- Commercial: Billing, Usage, Settings

The UI is an operational console: dense, stable, scan-friendly, and action-oriented. It must not become a marketing page.

## Domain Model

Conceptual objects:

- `Tenant`: id, name, plan, region, security settings, billing account.
- `TenantMembership`: tenant id, user id, tenant role, module grants, status, last activity.
- `TenantInvitation`: email, tenant role, module grants, workspace assignments, expiry, status.
- `Workspace`: tenant id, name, type, region, usage summary, membership summary.
- `WorkspaceMembership`: workspace id, user or principal id, role, status.
- `DeveloperResource`: service accounts, OAuth clients, API credentials, webhooks, tenant or workspace scopes.
- `Policy`: MFA, module access, workspace access, data residency, least privilege warnings.
- `BillingAccount`: plan, subscription status, invoices, payment portal state, usage limits.
- `AuditEvent`: actor, action, tenant scope, optional workspace scope, target, before/after summary, timestamp.

The implementation may map these concepts onto existing tables and endpoints, but product language and route boundaries remain tenant-first.

## Roles And Permissions

The V0 model uses predefined tenant roles plus module grants.

Tenant roles: Owner, Admin, Member, Viewer.

Module grants: Members, Workspaces, Developers, Policies, Security, Billing, Audit, Drive.

The role gives the broad default. Module grants refine what the user can administer. Workspace assignments refine where product access applies.

This avoids a heavy custom RBAC editor while supporting least privilege. A user can administer one module without becoming an accidental super-admin.

## Users V0

`Users` is the first complete workflow.

Main screen:

- tenant header and primary `Invite user` action;
- metrics for active users, pending invitations, suspended users, and policy risks;
- search by name or email;
- filters for role, workspace, module, and status;
- dense table with stable row heights;
- bulk actions for workspace assignment, module grants, suspension, and invite resend;
- right-side inspector for the selected user or invitation.

Inspector:

- identity summary;
- tenant role;
- module grants;
- workspace assignments;
- security state, including MFA policy status;
- recent audit events;
- available actions.

Invite flow:

1. Admin enters one or more email addresses.
2. Admin selects tenant role.
3. Admin selects module grants.
4. Admin optionally assigns workspaces.
5. UI previews policy warnings.
6. Admin confirms.
7. Backend creates invitations and audit events.
8. UI shows per-recipient success or recoverable errors.

Edit access flow:

1. Admin selects a user.
2. Admin changes role, modules, or workspace assignments.
3. UI previews risky changes.
4. Backend validates admin permission and tenant policy.
5. Backend rejects stale or unsafe transitions with explicit errors.
6. UI refreshes table and inspector from server state.

Suspend or reactivate flow:

1. Admin selects action.
2. UI requires an audit reason.
3. Backend enforces permission checks and last-owner protection.
4. Backend writes membership state and audit event.
5. UI confirms the new status.

## Other Modules V0

Other modules exist as route shells with useful summaries:

- `Overview`: tenant health, users, workspaces, invitations, security warnings, billing status.
- `Workspaces`: tenant workspace list, status, region, type, members, usage, create action when policy allows it.
- `Developers`: service accounts, OAuth clients, API credential warnings, scoped resource summaries.
- `Policies`: MFA, module grant, workspace access, data residency, and least privilege summaries.
- `Security`: MFA enforcement, risky users, suspended users, session and authentication posture.
- `Audit logs`: read-only tenant event stream with filters for actor, action, target, workspace, and date.
- `Billing`: plan, subscription status, invoices or portal link, payment and limit status.
- `Usage`: users, storage, API, and workspace usage summaries.
- `Settings`: tenant profile, display name, region or residency display, safe non-destructive settings.

## API Contracts

The app needs tenant-level read models. Existing workspace endpoints can be reused only behind adapters for truly workspace-scoped actions.

Required contract shape:

- `GET /api/v1/enterprise/context`: current tenant, current membership, available tenant memberships, and high-level permissions.
- `GET /api/v1/enterprise/overview`: Overview summary cards.
- `GET /api/v1/enterprise/users`: memberships, invitations, roles, module grants, workspace assignments, security state, and pagination metadata.
- `POST /api/v1/enterprise/invitations`: create tenant invitations.
- `PATCH /api/v1/enterprise/users/{userId}/access`: update tenant role, module grants, and workspace assignments.
- `POST /api/v1/enterprise/users/{userId}/suspend`: suspend a tenant membership with an audit reason.
- `POST /api/v1/enterprise/users/{userId}/reactivate`: reactivate a tenant membership with an audit reason.
- `GET /api/v1/enterprise/workspaces`: tenant workspace summaries.
- `GET /api/v1/enterprise/developers`: tenant developer resource summaries.
- `GET /api/v1/enterprise/policies`: tenant policy summaries and warnings.
- `GET /api/v1/enterprise/security`: security posture summaries.
- `GET /api/v1/enterprise/audit-events`: tenant audit events with filters.
- `GET /api/v1/enterprise/billing`: billing overview and portal availability.
- `GET /api/v1/enterprise/usage`: usage summaries.

Final route names may follow existing `account-service` conventions, but the boundary must remain tenant-first.

## Error Handling

Required recoverable error categories:

- unauthenticated;
- tenant context missing;
- tenant membership missing;
- permission denied;
- policy violation;
- last owner protected;
- stale version or conflict;
- invalid invite recipient;
- duplicate invitation;
- workspace assignment invalid;
- billing unavailable;
- retryable network or server failure.

UI handling:

- inline validation for form errors;
- row-level errors for batch operations;
- route-level retry states for failed loads;
- explicit permission-denied copy that names the missing grant when safe.

## Security And Audit

Security requirements:

- tenant context is mandatory for every enterprise route;
- backend authorizes every action regardless of UI visibility;
- dangerous actions require confirmation;
- destructive or access-reducing actions require an audit reason;
- removing or suspending the last owner is blocked;
- risky actions can require MFA step-up;
- audit events record actor, action, target, tenant, optional workspace, before/after summary, and timestamp;
- frontend never infers permission success from hidden controls;
- API responses do not leak data across tenants.

## Frontend Direction

The console should be restrained, dense, and efficient:

- stable sidebar and top context region;
- tables optimized for repeated admin work;
- right-side inspector for detail and edits;
- icon buttons for common tools where the existing UI supports them;
- no landing page or hero;
- no card-heavy promotional layout;
- no nested cards;
- no single-hue decorative palette;
- stable dimensions for tables, toolbars, buttons, and inspector panels;
- laptop-first admin ergonomics with mobile support for review and emergency actions.

## Testing

Frontend tests:

- helpers for permission labels, role/module displays, filtering, invitation payloads, access change payloads, and last-owner UI guards;
- Users route loading, empty, error, and populated states;
- invite dialog validation;
- inspector states for member, invitation, and suspended user.

Backend/API tests:

- tenant isolation and missing-membership denial;
- admin without Members grant cannot invite or edit users;
- last owner cannot be suspended or downgraded;
- invitation lifecycle and duplicate invitation handling;
- workspace assignment authorization;
- access change, suspension, and reactivation audit events.

Validation after implementation: targeted `enterprise-web` typecheck, lint, and tests; affected TypeScript checks through Nx; `cargo check --workspace` after backend changes; targeted Rust tests for new tenant admin contracts.

## Rollout Slices

1. Create `apps/enterprise-web` shell, route tree, tenant context, and navigation.
2. Define typed enterprise API client contracts.
3. Build read-only Users route from tenant-level data.
4. Add invitation flow.
5. Add access edit flow.
6. Add suspend and reactivate flow.
7. Add route shells for the other enterprise modules.
8. Add targeted tests and validation commands.
9. Wire root scripts and Nx project metadata when the app exists.

The first useful release is complete only when a customer admin can invite a user, grant module permissions, assign workspaces, and audit the change from tenant scope.

## References

- WorkOS, "The complete guide to user management for B2B SaaS": https://workos.com/blog/user-management-for-b2b-saas
- WorkOS, "Model your B2B SaaS with organizations": https://workos.com/blog/model-your-b2b-saas-with-organizations
- Clerk, "What is multi-tenancy and why it matters for B2B SaaS": https://clerk.com/blog/what-is-multi-tenancy-and-why-it-matters-for-B2B-SaaS
- Cerbos, "How to Implement Scalable Multitenant Authorization": https://www.cerbos.dev/blog/how-to-implement-scalable-multitenant-authorization
- BetterCloud, "SaaS security best practices": https://www.bettercloud.com/monitor/saas-security-best-practices/
- Sonar, "Audit Logging Best Practices": https://www.sonarsource.com/resources/library/audit-logging/
