# Enterprise Web Tenant Admin Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build `apps/enterprise-web` as the dedicated tenant administration console, with Users V0 complete and route shells for the other enterprise modules.

**Architecture:** Add a first-class Vite/React app rooted in tenant context, not workspace context. Extend `@nvbes/identity-client` with tenant-first contracts, then add matching `account-service` enterprise routes for context, users, invitations, access updates, suspension, reactivation, and module summaries.

**Tech Stack:** pnpm workspaces, Nx, Vite, React, TanStack Router, TanStack Query, zod, vite-plus tests, Axum, SQLx, PostgreSQL.

---

## Scope Check

This plan implements the first useful release from the spec: `enterprise-web` plus complete Users V0. Full future workflows for Developers, Policies, Security, Billing, Usage, and Settings stay as route shells with real summary states.

## File Structure

Frontend app:

- `apps/enterprise-web/package.json`, `project.json`, `tsconfig.json`, `vite.config.ts`, `index.html`, `public/icon.svg`
- `apps/enterprise-web/src/main.tsx`, `App.tsx`, `styles.css`
- `apps/enterprise-web/src/enterprise.router.tsx`, `enterprise.routes.tsx`, `enterprise.router.pages.tsx`
- `apps/enterprise-web/src/enterprise.api.ts`, `enterprise.queries.ts`, `enterprise.permissions.ts`, `enterprise.invites.ts`
- `apps/enterprise-web/src/components/EnterpriseLayout.tsx`, `EnterpriseSidebar.tsx`, `EnterpriseRouteState.tsx`
- `apps/enterprise-web/src/pages/OverviewPage.tsx`, `ModuleShellPage.tsx`, `UsersPage.tsx`, `UsersPage.table.tsx`, `UsersPage.inspector.tsx`, `UsersPage.invite.tsx`, `UsersPage.access.tsx`
- tests beside helpers: `enterprise.permissions.test.ts`, `enterprise.invites.test.ts`

Shared client:

- `libs/ts/identity-client/src/enterprise.schemas.ts`
- `libs/ts/identity-client/src/enterprise.client.ts`
- `libs/ts/identity-client/src/enterprise.helpers.test.ts`
- `libs/ts/identity-client/src/index.ts`

Workspace wiring:
- `scripts/dev-enterprise-web.sh`
- `scripts/dev-web.sh`
- `package.json`

## Task 1: Scaffold `enterprise-web`

**Files:**

- Create: `apps/enterprise-web/package.json`
- Create: `apps/enterprise-web/project.json`
- Create: `apps/enterprise-web/tsconfig.json`
- Create: `apps/enterprise-web/vite.config.ts`
- Create: `apps/enterprise-web/index.html`
- Create: `apps/enterprise-web/public/icon.svg`
- Create: `apps/enterprise-web/src/main.tsx`
- Create: `apps/enterprise-web/src/App.tsx`
- Create: `apps/enterprise-web/src/styles.css`

- [ ] **Step 1: Add app package metadata**

Use this exact package script surface:

```json
{
  "name": "nvbes-enterprise-web",
  "version": "0.1.0",
  "private": true,
  "type": "module",
  "scripts": {
    "dev": "node ./node_modules/vite-plus/bin/vp dev",
    "build": "tsc -b && node ./node_modules/vite-plus/bin/vp build",
    "check": "pnpm run typecheck",
    "preview": "node ./node_modules/vite-plus/bin/vp preview",
    "typecheck": "tsgo -b --pretty false",
    "lint": "node ./node_modules/vite-plus/bin/vp lint",
    "format": "node ./node_modules/vite-plus/bin/vp fmt --write",
    "format:check": "node ./node_modules/vite-plus/bin/vp fmt --check",
    "test": "node ./node_modules/vite-plus/bin/vp test"
  }
}
```

Use the dependency set from `apps/account-web/package.json`, keeping only packages used by the shell: `@nvbes/http-client`, `@nvbes/identity-client`, `@nvbes/web-runtime`, `@nvbes/web-ui`, Sentry, TanStack Query/Router, React, Tailwind, lucide, zod, class utilities, and Vite tooling.

- [ ] **Step 2: Add Nx and TS config**

`project.json`:

```json
{
  "name": "enterprise-web",
  "projectType": "application",
  "root": "apps/enterprise-web",
  "sourceRoot": "apps/enterprise-web/src",
  "tags": ["type:app", "domain:identity", "layer:app"]
}
```

`tsconfig.json` must extend `../../tsconfig.base.json`, include `src`, exclude `src/sw.ts`, and map `@/*`, `@nvbes/http-client`, `@nvbes/identity-client`, `@nvbes/web-runtime`, `@nvbes/web-runtime/posthog`, and `@nvbes/web-ui`.

- [ ] **Step 3: Add Vite and React root**

Copy `apps/account-web/vite.config.ts`, then change the Sentry project fallback to `enterprise-web` and the dev/preview port to `5175`. Keep the `/api` proxy pointing to the Account service base URL.

`src/main.tsx`:

```tsx
import React from 'react';
import ReactDOM from 'react-dom/client';
import { App } from './App';
import './styles.css';

const rootEl = document.getElementById('root');
if (!rootEl) throw new Error('Root element #root not found');

ReactDOM.createRoot(rootEl).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
```

`src/App.tsx`:

```tsx
import { createQueryClient, ErrorBoundary } from '@nvbes/web-runtime';
import { QueryClientProvider } from '@tanstack/react-query';
import { RouterProvider } from '@tanstack/react-router';
import { useState } from 'react';
import { router } from './enterprise.router';

export function App() {
  const [queryClient] = useState(() => createQueryClient());
  return (
    <ErrorBoundary name="enterprise-web">
      <QueryClientProvider client={queryClient}>
        <RouterProvider router={router} />
      </QueryClientProvider>
    </ErrorBoundary>
  );
}
```

- [ ] **Step 4: Verify and commit**

Run:

```bash
rtk pnpm exec nx show projects --json
rtk pnpm --dir apps/enterprise-web typecheck
```

Expected: `enterprise-web` appears and typecheck passes.

Commit:

```bash
git add apps/enterprise-web
git commit -m "feat(enterprise-web): scaffold app"
```

## Task 2: Add Routing, Layout, And Route Shells

**Files:**

- Create: `apps/enterprise-web/src/enterprise.router.tsx`
- Create: `apps/enterprise-web/src/enterprise.routes.tsx`
- Create: `apps/enterprise-web/src/enterprise.router.pages.tsx`
- Create: `apps/enterprise-web/src/components/EnterpriseLayout.tsx`
- Create: `apps/enterprise-web/src/components/EnterpriseSidebar.tsx`
- Create: `apps/enterprise-web/src/components/EnterpriseRouteState.tsx`
- Create: `apps/enterprise-web/src/pages/OverviewPage.tsx`
- Create: `apps/enterprise-web/src/pages/ModuleShellPage.tsx`

- [ ] **Step 1: Add router**

`enterprise.router.tsx`:

```tsx
import { createRootRoute, createRoute, createRouter, Outlet } from '@tanstack/react-router';
import { EnterpriseLayout } from './components/EnterpriseLayout';
import { createEnterpriseRoutes } from './enterprise.routes';

const rootRoute = createRootRoute({ component: Outlet });
export const enterpriseRoute = createRoute({
  getParentRoute: () => rootRoute,
  id: 'enterprise',
  component: EnterpriseLayout,
});
const routeTree = rootRoute.addChildren([enterpriseRoute.addChildren(createEnterpriseRoutes(enterpriseRoute))]);
export const router = createRouter({ routeTree });
declare module '@tanstack/react-router' {
  interface Register { router: typeof router; }
}
```

- [ ] **Step 2: Add navigation**

`EnterpriseSidebar.tsx` must expose:

```ts
export const enterpriseNavSections = [
  { title: 'Operate', items: ['Overview', 'Users', 'Workspaces', 'Developers'] },
  { title: 'Govern', items: ['Policies', 'Security', 'Audit logs'] },
  { title: 'Commercial', items: ['Billing', 'Usage', 'Settings'] },
] as const;
```

Use TanStack `Link`, lucide icons, and active styling consistent with `account-web`.

- [ ] **Step 3: Add route shells**

`ModuleShellPage` props:

```ts
export type ModuleShellPageProps = {
  title: string;
  summary: string;
  metrics: Array<{ label: string; value: string; tone?: 'neutral' | 'warning' }>;
};
```

Create routes for `/`, `/users`, `/workspaces`, `/developers`, `/policies`, `/security`, `/audit-logs`, `/billing`, `/usage`, and `/settings`. Use `OverviewPage` for `/` and `ModuleShellPage` for every route except Users until Task 5.

- [ ] **Step 4: Verify and commit**

Run:

```bash
rtk pnpm --dir apps/enterprise-web typecheck
rtk pnpm --dir apps/enterprise-web lint
```

Expected: both pass.

Commit:

```bash
git add apps/enterprise-web/src
git commit -m "feat(enterprise-web): add tenant navigation"
```

## Task 3: Add Typed Enterprise Client Contracts

**Files:**

- Create: `libs/ts/identity-client/src/enterprise.schemas.ts`
- Create: `libs/ts/identity-client/src/enterprise.client.ts`
- Create: `libs/ts/identity-client/src/enterprise.helpers.test.ts`
- Modify: `libs/ts/identity-client/src/index.ts`

- [ ] **Step 1: Write failing schema tests**

```ts
import { describe, expect, it } from 'vite-plus/test';
import { EnterpriseInvitationInputSchema, EnterpriseUsersResponseSchema } from './enterprise.schemas';

describe('enterprise client schemas', () => {
  it('parses users payloads', () => {
    const parsed = EnterpriseUsersResponseSchema.parse({
      users: [],
      invitations: [],
      roles: ['owner', 'admin', 'member', 'viewer'],
      module_grants: ['members', 'workspaces', 'developers', 'policies', 'security', 'billing', 'audit', 'drive'],
      page: { cursor: null, has_more: false },
    });
    expect(parsed.roles).toContain('owner');
  });

  it('validates invitation input', () => {
    expect(EnterpriseInvitationInputSchema.parse({
      emails: ['admin@example.com'],
      role: 'admin',
      module_grants: ['members'],
      workspace_ids: ['workspace_123'],
    }).role).toBe('admin');
  });
});
```

Run:

```bash
rtk pnpm --dir libs/ts/identity-client test -- enterprise.helpers.test.ts
```

Expected: fail because schemas do not exist.

- [ ] **Step 2: Implement schemas and client methods**

`enterprise.schemas.ts` must define:

```ts
export const EnterpriseRoleSchema = z.enum(['owner', 'admin', 'member', 'viewer']);
export const EnterpriseModuleGrantSchema = z.enum(['members', 'workspaces', 'developers', 'policies', 'security', 'billing', 'audit', 'drive']);
```

Add response schemas for context, overview, users, workspaces, developers, policies, security, audit events, billing, and usage. Add input schemas for invitations, access update, suspend, and reactivate.

`enterprise.client.ts` must export functions that attach methods to `IdentityClient`: `getEnterpriseContext`, `getEnterpriseUsers`, `createEnterpriseInvitations`, `updateEnterpriseUserAccess`, `suspendEnterpriseUser`, `reactivateEnterpriseUser`, and summary getters.

- [ ] **Step 3: Verify and commit**

Run:

```bash
rtk pnpm --dir libs/ts/identity-client test -- enterprise.helpers.test.ts
rtk pnpm --dir libs/ts/identity-client typecheck
```

Expected: tests and typecheck pass.

Commit:

```bash
git add libs/ts/identity-client/src
git commit -m "feat(identity-client): add enterprise contracts"
```

## Task 4: Add Frontend Helper Logic

**Files:**

- Create: `apps/enterprise-web/src/enterprise.permissions.ts`
- Create: `apps/enterprise-web/src/enterprise.permissions.test.ts`
- Create: `apps/enterprise-web/src/enterprise.invites.ts`
- Create: `apps/enterprise-web/src/enterprise.invites.test.ts`

- [ ] **Step 1: Write failing helper tests**

```ts
import { describe, expect, it } from 'vite-plus/test';
import { canManageUsers, describeModuleGrant, isLastOwnerRemoval } from './enterprise.permissions';

describe('enterprise permissions', () => {
  it('allows owners and members-admins to manage users', () => {
    expect(canManageUsers({ role: 'owner', module_grants: [] })).toBe(true);
    expect(canManageUsers({ role: 'admin', module_grants: ['members'] })).toBe(true);
  });
  it('blocks last owner removal', () => {
    expect(isLastOwnerRemoval({ currentOwnerCount: 1, selectedRole: 'owner', nextRole: 'admin' })).toBe(true);
  });
  it('labels grants', () => {
    expect(describeModuleGrant('members')).toBe('Members');
  });
});
```

Run:

```bash
rtk pnpm --dir apps/enterprise-web test -- enterprise.permissions.test.ts
```

Expected: fail because helpers do not exist.

- [ ] **Step 2: Implement helpers**

```ts
export function canManageUsers(input: { role: EnterpriseRole; module_grants: EnterpriseModuleGrant[] }) {
  return input.role === 'owner' || (input.role === 'admin' && input.module_grants.includes('members'));
}
export function isLastOwnerRemoval(input: { currentOwnerCount: number; selectedRole: EnterpriseRole; nextRole: EnterpriseRole }) {
  return input.currentOwnerCount <= 1 && input.selectedRole === 'owner' && input.nextRole !== 'owner';
}
```

`enterprise.invites.ts` must split comma/newline email input, trim values, reject empty recipient lists, and build `EnterpriseInvitationInput`.

- [ ] **Step 3: Verify and commit**

Run:

```bash
rtk pnpm --dir apps/enterprise-web test -- enterprise.permissions.test.ts enterprise.invites.test.ts
rtk pnpm --dir apps/enterprise-web typecheck
```

Expected: tests and typecheck pass.

Commit:

```bash
git add apps/enterprise-web/src/enterprise.permissions* apps/enterprise-web/src/enterprise.invites*
git commit -m "feat(enterprise-web): add access helpers"
```

## Task 5: Build Users V0 Frontend

**Files:**

- Create: `apps/enterprise-web/src/enterprise.api.ts`
- Create: `apps/enterprise-web/src/enterprise.queries.ts`
- Create: `apps/enterprise-web/src/pages/UsersPage.tsx`
- Create: `apps/enterprise-web/src/pages/UsersPage.table.tsx`
- Create: `apps/enterprise-web/src/pages/UsersPage.inspector.tsx`
- Create: `apps/enterprise-web/src/pages/UsersPage.invite.tsx`
- Create: `apps/enterprise-web/src/pages/UsersPage.access.tsx`
- Modify: `apps/enterprise-web/src/enterprise.routes.tsx`

- [ ] **Step 1: Add API and queries**

```ts
export const enterpriseQueryKeys = {
  context: ['enterprise', 'context'] as const,
  users: ['enterprise', 'users'] as const,
};
```

`enterprise.api.ts` exports `enterpriseClient = identityClient`.

- [ ] **Step 2: Add read-only Users route**

Render loading, error, empty, and populated states. Table columns: name/email, role, module grants, workspaces, status, last active. Inspector states: none selected, member, invitation, suspended member.

- [ ] **Step 3: Add mutations**

Invite dialog includes email input, role select, module grants, workspace assignments, warning preview, submit, and per-recipient errors. Access panel edits role, grants, and workspaces. Suspend/reactivate require an audit reason and invalidate `enterpriseQueryKeys.users`.

- [ ] **Step 4: Verify and commit**

Run:

```bash
rtk pnpm --dir apps/enterprise-web typecheck
rtk pnpm --dir apps/enterprise-web lint
```

Expected: both pass.

Commit:

```bash
git add apps/enterprise-web/src
git commit -m "feat(enterprise-web): add users admin workflow"
```

## Task 6: Execute Backend/API Plan

**Files:**

- Follow: `docs/superpowers/plans/2026-06-12-enterprise-web-tenant-admin-api.md`

- [ ] **Step 1: Complete backend enterprise routes**

Run the companion backend/API plan before treating Users V0 as integrated. It creates `identity.domains.enterprise.*`, wires the router, adds guardrail tests, and validates with `cargo check --workspace`.

- [ ] **Step 2: Confirm client/backend route alignment**

Compare `libs/ts/identity-client/src/enterprise.client.ts` with `apps/account-service/src/identity.domains.enterprise.routes.rs`. Every client method must have a matching backend route and response schema.

- [ ] **Step 3: Commit alignment fixes**

```bash
git add libs/ts/identity-client/src apps/account-service/src/identity.domains.enterprise.* apps/account-service/src/identity.domains.mod.rs
git commit -m "fix: align enterprise admin client and API"
```

Skip the commit if no alignment fixes are needed.

## Task 7: Wire Workspace Scripts And Final Checks

**Files:**

- Create: `scripts/dev-enterprise-web.sh`
- Modify: `scripts/dev-web.sh`
- Modify: `package.json`
- Modify: `apps/enterprise-web/src/pages/ModuleShellPage.tsx`

- [ ] **Step 1: Add dev script**

```bash
#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib/test-env.sh"
cd "$ROOT_DIR"
exec pnpm --dir apps/enterprise-web dev
```

Make it executable.

- [ ] **Step 2: Update root scripts**

Add `dev:enterprise-web`. Include `enterprise-web` in `build:web`, `check:web`, `lint:web`, and `dev:web`.

- [ ] **Step 3: Run final verification**

```bash
rtk pnpm exec nx run-many -t lint,typecheck --projects=enterprise-web,identity-client
rtk cargo test -p nvbes-account-service enterprise
rtk cargo check --workspace
rtk pnpm --dir apps/enterprise-web dev
```

Open `http://localhost:5175` and verify navigation, Users route, route shells, no console errors, and stable desktop/mobile layout.

- [ ] **Step 4: Commit**

```bash
git add scripts/dev-enterprise-web.sh scripts/dev-web.sh package.json apps/enterprise-web
git commit -m "chore: wire enterprise web checks"
```
