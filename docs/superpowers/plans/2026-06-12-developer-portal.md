# nvbes Developer Portal Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a production-connected developer site and portal for nvbes Identity with OAuth app management, fine-grained developer RBAC, token tools, logs, webhooks, OpenAPI, SDK links, and quickstarts.

**Architecture:** Add `apps/console-web` as a standalone React/Vite app and add a `developer` domain inside `apps/account-service` as the backend product facade. The developer domain enforces developer permissions before delegating to existing OAuth, session, security event, and tenant primitives. Each task below leaves the repository in a compiling or narrowly testable state.

**Tech Stack:** Rust, Axum, SQLx, PostgreSQL, utoipa OpenAPI, pnpm workspaces, Nx, React, TypeScript, Vite, TanStack Router, TanStack Query, `@nvbes/web-ui`, `@nvbes/web-runtime`, lucide-react.

---

## File Structure

Frontend app:

- Create `apps/console-web/package.json`: npm scripts and dependencies for Nx inferred targets.
- Create `apps/console-web/project.json`: Nx project metadata.
- Create `apps/console-web/index.html`: Vite root HTML.
- Create `apps/console-web/tsconfig.json`: app TypeScript project config.
- Create `apps/console-web/vite.config.ts`: Vite Plus config with React, Tailwind, devtools JSON, and `/api` proxy.
- Create `apps/console-web/src/main.tsx`: React entrypoint.
- Create `apps/console-web/src/App.tsx`: providers and router mount.
- Create `apps/console-web/src/styles.css`: Tailwind theme entry and developer app tokens.
- Create `apps/console-web/src/developer.router.tsx`: route tree.
- Create `apps/console-web/src/developer.api.ts`: typed API functions.
- Create `apps/console-web/src/developer.schemas.ts`: Zod response and form schemas.
- Create `apps/console-web/src/developer.fixtures.ts`: local docs content and deterministic UI fallback data for docs-only views.
- Create `apps/console-web/src/layouts/DeveloperPublicLayout.tsx`: public docs layout.
- Create `apps/console-web/src/layouts/DeveloperPortalLayout.tsx`: authenticated portal shell.
- Create page files under `apps/console-web/src/pages/`: `DeveloperHomePage.tsx`, `QuickstartPage.tsx`, `ApiReferencePage.tsx`, `PortalOverviewPage.tsx`, `PortalAppsPage.tsx`, `PortalAppDetailPage.tsx`, `PortalTokenInspectorPage.tsx`, `PortalOAuthPlaygroundPage.tsx`, `PortalLogsPage.tsx`, `PortalWebhooksPage.tsx`, `PortalRolesPage.tsx`.

Root web scripts:

- Modify `package.json`: add `console-web` to root web build/check/lint project lists and add `dev:console-web`.
- Create `scripts/dev-console-web.sh`: start the new app consistently with existing web scripts.

Backend domain:

- Create `apps/account-service/migrations/0008_developer_portal.sql`: developer role assignments, webhook endpoints, subscriptions, and deliveries.
- Create `apps/account-service/src/identity.domains.developer.mod.rs`: module declarations and router.
- Create `apps/account-service/src/identity.domains.developer.types.rs`: request/response DTOs and OpenAPI schemas.
- Create `apps/account-service/src/identity.domains.developer.rbac.rs`: permission matrix and enforcement helpers.
- Create `apps/account-service/src/identity.domains.developer.rbac.db.rs`: SQLx role assignment queries.
- Create `apps/account-service/src/identity.domains.developer.apps.routes.rs`: `/developer/apps` route handlers.
- Create `apps/account-service/src/identity.domains.developer.apps.service.rs`: OAuth facade for developer apps.
- Create `apps/account-service/src/identity.domains.developer.tokens.routes.rs`: token inspector and OAuth playground exchange route handlers.
- Create `apps/account-service/src/identity.domains.developer.logs.routes.rs`: filtered logs route handlers.
- Create `apps/account-service/src/identity.domains.developer.webhooks.routes.rs`: webhook endpoint route handlers.
- Create `apps/account-service/src/identity.domains.developer.webhooks.service.rs`: webhook endpoint, subscription, delivery, and signing helpers.
- Create `apps/account-service/src/identity.domains.developer.tests.rs`: focused RBAC and facade tests.
- Modify `apps/account-service/src/identity.domains.mod.rs`: expose and merge the developer router.
- Modify `apps/account-service/src/identity.http.openapi.rs`: add developer tag and paths.

OpenAPI and docs:

- Modify `libs/ts/identity-sdk-core/openapi.json`: regenerated OpenAPI.
- Modify `libs/ts/identity-sdk-core/src/types.gen.ts`: regenerated TypeScript OpenAPI types.
- Modify `libs/ts/identity-sdk/README.md`: add developer portal and SDK quickstart links.
- Create `docs/api/identity-developer-portal.md`: backend route and permission reference.

---

## Task 1: Scaffold `console-web`

**Files:**

- Create: `apps/console-web/package.json`
- Create: `apps/console-web/project.json`
- Create: `apps/console-web/index.html`
- Create: `apps/console-web/tsconfig.json`
- Create: `apps/console-web/vite.config.ts`
- Create: `apps/console-web/src/main.tsx`
- Create: `apps/console-web/src/App.tsx`
- Create: `apps/console-web/src/styles.css`
- Create: `apps/console-web/src/developer.router.tsx`
- Modify: `package.json`
- Create: `scripts/dev-console-web.sh`

- [ ] **Step 1: Create the package manifest**

Create `apps/console-web/package.json`:

```json
{
  "name": "nvbes-console-web",
  "version": "0.1.0",
  "private": true,
  "type": "module",
  "scripts": {
    "dev": "node ./node_modules/vite-plus/bin/vp dev",
    "build": "tsc -b && node ./node_modules/vite-plus/bin/vp build",
    "check": "pnpm run typecheck",
    "format": "node ./node_modules/vite-plus/bin/vp fmt --write",
    "format:check": "node ./node_modules/vite-plus/bin/vp fmt --check",
    "lint": "node ./node_modules/vite-plus/bin/vp lint",
    "preview": "node ./node_modules/vite-plus/bin/vp preview",
    "typecheck": "tsgo -b --pretty false"
  },
  "dependencies": {
    "@base-ui/react": "^1.4.1",
    "@fontsource-variable/geist": "^5.2.9",
    "@nvbes/http-client": "workspace:*",
    "@nvbes/identity-client": "workspace:*",
    "@nvbes/identity-sdk-web": "workspace:*",
    "@nvbes/web-runtime": "workspace:*",
    "@nvbes/web-ui": "workspace:*",
    "@tanstack/react-query": "catalog:",
    "@tanstack/react-query-devtools": "catalog:",
    "@tanstack/react-router": "catalog:",
    "@tanstack/react-router-devtools": "catalog:",
    "class-variance-authority": "^0.7.1",
    "clsx": "^2.1.1",
    "lucide-react": "^1.16.0",
    "react": "catalog:",
    "react-dom": "catalog:",
    "shadcn": "^4.7.0",
    "tailwind-merge": "^3.6.0",
    "tw-animate-css": "^1.4.0",
    "zod": "^3.25.76"
  },
  "devDependencies": {
    "@tailwindcss/vite": "catalog:",
    "@types/react": "^19.2.14",
    "@types/react-dom": "^19.2.3",
    "@typescript/native-preview": "catalog:",
    "@vitejs/plugin-react": "catalog:",
    "tailwindcss": "catalog:",
    "typescript": "catalog:",
    "vite": "catalog:",
    "vite-plugin-devtools-json": "^1.0.0",
    "vite-plus": "catalog:"
  }
}
```

- [ ] **Step 2: Create the Nx project metadata**

Create `apps/console-web/project.json`:

```json
{
  "name": "console-web",
  "projectType": "application",
  "root": "apps/console-web",
  "sourceRoot": "apps/console-web/src",
  "tags": ["type:app", "domain:identity", "layer:app"]
}
```

- [ ] **Step 3: Create TypeScript and Vite entry files**

Create `apps/console-web/tsconfig.json`:

```json
{
  "extends": "../../tsconfig.base.json",
  "compilerOptions": {
    "baseUrl": ".",
    "paths": {
      "@/*": ["src/*"]
    },
    "types": ["vite/client", "vite-plus/client"]
  },
  "include": ["src", "vite.config.ts"],
  "references": []
}
```

Create `apps/console-web/index.html`:

```html
<!doctype html>
<html lang="fr">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <meta name="description" content="nvbes Developers: API Identity, SDKs, OAuth apps, webhooks, logs, and integration tools." />
    <title>nvbes Developers</title>
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
```

Create `apps/console-web/vite.config.ts`:

```ts
import tailwindcss from '@tailwindcss/vite';
import react from '@vitejs/plugin-react';
import devtoolsJson from 'vite-plugin-devtools-json';
import { defineConfig, loadEnv } from 'vite-plus';

export default defineConfig(({ mode }) => {
  const localEnv = loadEnv(mode, process.cwd(), '');
  const rootEnv = loadEnv(mode, '../../', '');
  const accountServiceProxyTarget =
    process.env.VITE_ACCOUNT_SERVICE_PROXY_TARGET ||
    localEnv.VITE_ACCOUNT_SERVICE_PROXY_TARGET ||
    rootEnv.VITE_ACCOUNT_SERVICE_PROXY_TARGET ||
    'http://localhost:8080';

  return {
    plugins: [react(), tailwindcss(), devtoolsJson()],
    server: {
      port: 5175,
      proxy: {
        '/api': accountServiceProxyTarget,
        '/developer': accountServiceProxyTarget,
        '/oauth': accountServiceProxyTarget,
      },
    },
  };
});
```

- [ ] **Step 4: Create a minimal route tree**

Create `apps/console-web/src/developer.router.tsx`:

```tsx
import { createRootRoute, createRoute, createRouter, Link, Outlet } from '@tanstack/react-router';

function RootShell() {
  return <Outlet />;
}

function HomePage() {
  return (
    <main className="min-h-screen bg-background text-foreground">
      <section className="mx-auto flex min-h-screen w-full max-w-6xl flex-col gap-8 px-6 py-10">
        <nav className="flex items-center justify-between border-b border-border pb-4">
          <Link to="/" className="font-heading text-lg font-semibold">
            nvbes Developers
          </Link>
          <Link to="/portal" className="text-sm font-medium text-primary">
            Open portal
          </Link>
        </nav>
        <div className="grid gap-6 md:grid-cols-[1.1fr_0.9fr]">
          <div className="flex flex-col justify-center gap-5">
            <p className="text-sm font-semibold uppercase tracking-[0.18em] text-muted-foreground">
              Identity platform
            </p>
            <h1 className="max-w-3xl text-4xl font-semibold tracking-normal md:text-6xl">
              Build secure OAuth apps with nvbes Identity.
            </h1>
            <p className="max-w-2xl text-base leading-7 text-muted-foreground">
              Configure clients, redirects, webhooks, logs, token inspection, SDKs, and quickstarts
              from one developer console.
            </p>
            <div className="flex flex-wrap gap-3">
              <Link to="/portal/apps" className="rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground">
                Create app
              </Link>
              <Link to="/quickstarts/react" className="rounded-md border border-border px-4 py-2 text-sm font-medium">
                React quickstart
              </Link>
            </div>
          </div>
          <div className="grid content-start gap-3 rounded-lg border border-border bg-card p-4">
            {['OAuth apps', 'Token inspector', 'Webhooks', 'Filtered logs'].map((label) => (
              <div key={label} className="rounded-md border border-border bg-background p-4 text-sm font-medium">
                {label}
              </div>
            ))}
          </div>
        </div>
      </section>
    </main>
  );
}

function PortalEntryPreview() {
  return (
    <main className="min-h-screen bg-background px-6 py-8 text-foreground">
      <div className="mx-auto max-w-6xl">
        <h1 className="text-2xl font-semibold">Developer portal</h1>
        <p className="mt-2 text-sm text-muted-foreground">
          Connected portal routes are introduced by Tasks 8, 9, and 10 in this plan.
        </p>
      </div>
    </main>
  );
}

const rootRoute = createRootRoute({ component: RootShell });

const homeRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/',
  component: HomePage,
});

const portalRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/portal',
  component: PortalEntryPreview,
});

const routeTree = rootRoute.addChildren([homeRoute, portalRoute]);

export const router = createRouter({ routeTree });

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router;
  }
}
```

- [ ] **Step 5: Create React app entrypoint**

Create `apps/console-web/src/App.tsx`:

```tsx
import { createQueryClient, ErrorBoundary } from '@nvbes/web-runtime';
import { QueryClientProvider } from '@tanstack/react-query';
import { ReactQueryDevtools } from '@tanstack/react-query-devtools';
import { RouterProvider } from '@tanstack/react-router';
import { TanStackRouterDevtools } from '@tanstack/react-router-devtools';
import { useState } from 'react';
import { router } from './developer.router';

export default function App() {
  const [queryClient] = useState(() => createQueryClient());

  return (
    <ErrorBoundary name="console-web">
      <QueryClientProvider client={queryClient}>
        <RouterProvider router={router} />
        {import.meta.env.DEV ? (
          <>
            <ReactQueryDevtools initialIsOpen={false} />
            <TanStackRouterDevtools router={router} position="bottom-right" />
          </>
        ) : null}
      </QueryClientProvider>
    </ErrorBoundary>
  );
}
```

Create `apps/console-web/src/main.tsx`:

```tsx
import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import './styles.css';

const rootEl = document.getElementById('root');
if (!rootEl) {
  throw new Error('Root element #root not found');
}

ReactDOM.createRoot(rootEl).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
```

Create `apps/console-web/src/styles.css`:

```css
@import 'tailwindcss';
@import 'tw-animate-css';
@import 'shadcn/tailwind.css';
@import '@fontsource-variable/geist';

@source '../../../libs/ts/web-runtime/src';
@source '../../../libs/ts/web-ui/src';
@source './';

@theme inline {
  --font-heading: var(--font-sans);
  --font-sans: 'Geist Variable', sans-serif;
  --color-ring: var(--ring);
  --color-input: var(--input);
  --color-border: var(--border);
  --color-muted-foreground: var(--muted-foreground);
  --color-muted: var(--muted);
  --color-primary-foreground: var(--primary-foreground);
  --color-primary: var(--primary);
  --color-card-foreground: var(--card-foreground);
  --color-card: var(--card);
  --color-foreground: var(--foreground);
  --color-background: var(--background);
  --radius-sm: calc(var(--radius) * 0.6);
  --radius-md: calc(var(--radius) * 0.8);
  --radius-lg: var(--radius);
}

:root {
  --background: oklch(0.985 0.003 214);
  --foreground: oklch(0.15 0.01 235);
  --card: oklch(1 0 0);
  --card-foreground: oklch(0.15 0.01 235);
  --primary: oklch(0.5 0.11 184);
  --primary-foreground: oklch(0.98 0.01 180);
  --muted: oklch(0.94 0.006 210);
  --muted-foreground: oklch(0.46 0.03 220);
  --border: oklch(0.88 0.01 214);
  --input: oklch(0.88 0.01 214);
  --ring: oklch(0.65 0.05 190);
  --radius: 0.5rem;
}

@layer base {
  * {
    @apply border-border outline-ring/50;
  }

  html {
    @apply font-sans;
  }

  body {
    @apply bg-background text-foreground;
  }
}
```

- [ ] **Step 6: Wire root scripts**

Modify root `package.json`:

```json
{
  "scripts": {
    "dev:console-web": "bash scripts/dev-console-web.sh",
    "build:web": "pnpm --dir apps/cloud-web build && pnpm --dir apps/account-web build && pnpm --dir apps/console-web build"
  }
}
```

Also add `console-web` to root `build`, `check:web`, `format:check`, and `lint:web` project lists next to `account-web`.

Create `scripts/dev-console-web.sh`:

```bash
#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
pnpm --dir apps/console-web dev
```

Run:

```bash
rtk chmod +x scripts/dev-console-web.sh
```

- [ ] **Step 7: Verify Nx discovers the app**

Run:

```bash
rtk pnpm exec nx show project console-web --json
```

Expected: JSON output with `"root":"apps/console-web"` and targets including `build`, `lint`, and `typecheck`.

- [ ] **Step 8: Verify frontend checks**

Run:

```bash
rtk pnpm exec nx run console-web:typecheck
rtk pnpm exec nx run console-web:lint
```

Expected: both commands pass.

- [ ] **Step 9: Commit**

```bash
rtk git add apps/console-web package.json scripts/dev-console-web.sh
rtk git commit -m "feat(developer): scaffold console web app"
```

---

## Task 2: Add Developer RBAC Persistence And Pure Permission Mapping

**Files:**

- Create: `apps/account-service/migrations/0008_developer_portal.sql`
- Create: `apps/account-service/src/identity.domains.developer.mod.rs`
- Create: `apps/account-service/src/identity.domains.developer.types.rs`
- Create: `apps/account-service/src/identity.domains.developer.rbac.rs`
- Create: `apps/account-service/src/identity.domains.developer.tests.rs`
- Modify: `apps/account-service/src/identity.domains.mod.rs`

- [ ] **Step 1: Write failing RBAC tests**

Create `apps/account-service/src/identity.domains.developer.tests.rs`:

```rust
use super::rbac::{DeveloperPermission, DeveloperRole, permissions_for_role};

#[test]
fn developer_admin_has_all_developer_permissions() {
    let permissions = permissions_for_role(DeveloperRole::DeveloperAdmin);

    assert!(permissions.contains(&DeveloperPermission::AppsCreate));
    assert!(permissions.contains(&DeveloperPermission::WebhooksManage));
    assert!(permissions.contains(&DeveloperPermission::LogsRead));
    assert!(permissions.contains(&DeveloperPermission::RbacManage));
}

#[test]
fn integration_tester_can_use_tools_without_mutating_apps() {
    let permissions = permissions_for_role(DeveloperRole::IntegrationTester);

    assert!(permissions.contains(&DeveloperPermission::TokensInspect));
    assert!(permissions.contains(&DeveloperPermission::OAuthPlayground));
    assert!(permissions.contains(&DeveloperPermission::AppsRead));
    assert!(!permissions.contains(&DeveloperPermission::AppsCreate));
    assert!(!permissions.contains(&DeveloperPermission::WebhooksManage));
}

#[test]
fn docs_viewer_has_only_docs_access() {
    let permissions = permissions_for_role(DeveloperRole::DocsViewer);

    assert_eq!(permissions, vec![DeveloperPermission::DocsRead]);
}
```

- [ ] **Step 2: Run tests to verify failure**

Run:

```bash
rtk cargo test -p nvbes-account-service developer_ --lib
```

Expected: fail because the developer module does not exist.

- [ ] **Step 3: Add SQL migration**

Create `apps/account-service/migrations/0008_developer_portal.sql`:

```sql
CREATE TYPE developer_role AS ENUM (
  'developer_admin',
  'app_manager',
  'webhook_manager',
  'log_viewer',
  'integration_tester',
  'docs_viewer'
);

CREATE TABLE developer_role_assignments (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  principal_id uuid NOT NULL REFERENCES principals(id) ON DELETE CASCADE,
  role developer_role NOT NULL,
  assigned_by uuid REFERENCES principals(id) ON DELETE SET NULL,
  created_at timestamptz NOT NULL DEFAULT now(),
  revoked_at timestamptz,
  UNIQUE (tenant_id, principal_id, role)
);

CREATE INDEX idx_developer_role_assignments_tenant_principal
  ON developer_role_assignments (tenant_id, principal_id)
  WHERE revoked_at IS NULL;

CREATE TABLE developer_webhook_endpoints (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  name text NOT NULL,
  url text NOT NULL,
  signing_secret_ciphertext text NOT NULL,
  signing_secret_last4 text NOT NULL,
  status text NOT NULL DEFAULT 'active',
  created_by uuid REFERENCES principals(id) ON DELETE SET NULL,
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now(),
  revoked_at timestamptz,
  CONSTRAINT developer_webhook_endpoints_status_check
    CHECK (status IN ('active', 'paused', 'revoked'))
);

CREATE INDEX idx_developer_webhook_endpoints_tenant
  ON developer_webhook_endpoints (tenant_id)
  WHERE revoked_at IS NULL;

CREATE TABLE developer_webhook_subscriptions (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  endpoint_id uuid NOT NULL REFERENCES developer_webhook_endpoints(id) ON DELETE CASCADE,
  event_type text NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now(),
  UNIQUE (endpoint_id, event_type),
  CONSTRAINT developer_webhook_subscriptions_event_type_check
    CHECK (event_type IN ('user.created', 'login.failed', 'session.revoked', 'client.created'))
);

CREATE TABLE developer_webhook_deliveries (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  endpoint_id uuid NOT NULL REFERENCES developer_webhook_endpoints(id) ON DELETE CASCADE,
  tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  event_type text NOT NULL,
  event_id uuid NOT NULL,
  status text NOT NULL,
  attempt_count integer NOT NULL DEFAULT 0,
  response_status integer,
  error_message text,
  created_at timestamptz NOT NULL DEFAULT now(),
  delivered_at timestamptz,
  CONSTRAINT developer_webhook_deliveries_status_check
    CHECK (status IN ('pending', 'delivered', 'failed'))
);

CREATE INDEX idx_developer_webhook_deliveries_endpoint_created
  ON developer_webhook_deliveries (endpoint_id, created_at DESC);

CREATE INDEX idx_developer_webhook_deliveries_tenant_event
  ON developer_webhook_deliveries (tenant_id, event_type, created_at DESC);
```

- [ ] **Step 4: Add RBAC types and permission mapping**

Create `apps/account-service/src/identity.domains.developer.rbac.rs`:

```rust
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DeveloperRole {
    DeveloperAdmin,
    AppManager,
    WebhookManager,
    LogViewer,
    IntegrationTester,
    DocsViewer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
pub enum DeveloperPermission {
    #[serde(rename = "developer.apps.read")]
    AppsRead,
    #[serde(rename = "developer.apps.create")]
    AppsCreate,
    #[serde(rename = "developer.apps.update_redirects")]
    AppsUpdateRedirects,
    #[serde(rename = "developer.apps.revoke")]
    AppsRevoke,
    #[serde(rename = "developer.webhooks.read")]
    WebhooksRead,
    #[serde(rename = "developer.webhooks.manage")]
    WebhooksManage,
    #[serde(rename = "developer.logs.read")]
    LogsRead,
    #[serde(rename = "developer.tokens.inspect")]
    TokensInspect,
    #[serde(rename = "developer.oauth.playground")]
    OAuthPlayground,
    #[serde(rename = "developer.rbac.manage")]
    RbacManage,
    #[serde(rename = "developer.docs.read")]
    DocsRead,
}

pub fn permissions_for_role(role: DeveloperRole) -> Vec<DeveloperPermission> {
    match role {
        DeveloperRole::DeveloperAdmin => vec![
            DeveloperPermission::AppsRead,
            DeveloperPermission::AppsCreate,
            DeveloperPermission::AppsUpdateRedirects,
            DeveloperPermission::AppsRevoke,
            DeveloperPermission::WebhooksRead,
            DeveloperPermission::WebhooksManage,
            DeveloperPermission::LogsRead,
            DeveloperPermission::TokensInspect,
            DeveloperPermission::OAuthPlayground,
            DeveloperPermission::RbacManage,
            DeveloperPermission::DocsRead,
        ],
        DeveloperRole::AppManager => vec![
            DeveloperPermission::AppsRead,
            DeveloperPermission::AppsCreate,
            DeveloperPermission::AppsUpdateRedirects,
            DeveloperPermission::AppsRevoke,
            DeveloperPermission::DocsRead,
        ],
        DeveloperRole::WebhookManager => vec![
            DeveloperPermission::WebhooksRead,
            DeveloperPermission::WebhooksManage,
            DeveloperPermission::AppsRead,
            DeveloperPermission::DocsRead,
        ],
        DeveloperRole::LogViewer => vec![
            DeveloperPermission::LogsRead,
            DeveloperPermission::AppsRead,
            DeveloperPermission::WebhooksRead,
            DeveloperPermission::DocsRead,
        ],
        DeveloperRole::IntegrationTester => vec![
            DeveloperPermission::TokensInspect,
            DeveloperPermission::OAuthPlayground,
            DeveloperPermission::AppsRead,
            DeveloperPermission::DocsRead,
        ],
        DeveloperRole::DocsViewer => vec![DeveloperPermission::DocsRead],
    }
}

impl DeveloperRole {
    pub fn as_db_str(self) -> &'static str {
        match self {
            DeveloperRole::DeveloperAdmin => "developer_admin",
            DeveloperRole::AppManager => "app_manager",
            DeveloperRole::WebhookManager => "webhook_manager",
            DeveloperRole::LogViewer => "log_viewer",
            DeveloperRole::IntegrationTester => "integration_tester",
            DeveloperRole::DocsViewer => "docs_viewer",
        }
    }
}
```

Create `apps/account-service/src/identity.domains.developer.types.rs`:

```rust
use serde::Serialize;
use utoipa::ToSchema;

use super::rbac::{DeveloperPermission, DeveloperRole};

#[derive(Debug, Serialize, ToSchema)]
pub struct DeveloperMeResponse {
    pub tenant_id: String,
    pub roles: Vec<DeveloperRole>,
    pub permissions: Vec<DeveloperPermission>,
}
```

- [ ] **Step 5: Add module declarations and empty router**

Create `apps/account-service/src/identity.domains.developer.mod.rs`:

```rust
use axum::Router;

use crate::app::AppState;

#[path = "identity.domains.developer.types.rs"]
pub mod types;
#[path = "identity.domains.developer.rbac.rs"]
pub mod rbac;

#[cfg(test)]
#[path = "identity.domains.developer.tests.rs"]
mod tests;

pub fn router(_state: &AppState) -> Router<AppState> {
    Router::new()
}
```

Modify `apps/account-service/src/identity.domains.mod.rs`:

```rust
#[path = "identity.domains.developer.mod.rs"]
pub mod developer;
```

Add `.merge(developer::router(state))` in `router`.

- [ ] **Step 6: Run RBAC tests**

Run:

```bash
rtk cargo test -p nvbes-account-service developer_ --lib
```

Expected: pass.

- [ ] **Step 7: Commit**

```bash
rtk git add apps/account-service/migrations/0008_developer_portal.sql apps/account-service/src/identity.domains.developer.*.rs apps/account-service/src/identity.domains.mod.rs
rtk git commit -m "feat(identity): add developer rbac foundation"
```

---

## Task 3: Implement Developer Role Loading And `/developer/me`

**Files:**

- Create: `apps/account-service/src/identity.domains.developer.rbac.db.rs`
- Create: `apps/account-service/src/identity.domains.developer.routes.rs`
- Modify: `apps/account-service/src/identity.domains.developer.mod.rs`
- Modify: `apps/account-service/src/identity.domains.developer.types.rs`
- Modify: `apps/account-service/src/identity.http.openapi.rs`

- [ ] **Step 1: Add database loader**

Create `apps/account-service/src/identity.domains.developer.rbac.db.rs`:

```rust
use std::collections::HashSet;

use sqlx::PgPool;
use uuid::Uuid;

use crate::http::error::AppError;

use super::rbac::{DeveloperPermission, DeveloperRole, permissions_for_role};

pub async fn load_developer_roles(
    db: &PgPool,
    tenant_id: Uuid,
    principal_id: Uuid,
) -> Result<Vec<DeveloperRole>, AppError> {
    let rows = sqlx::query_scalar::<_, String>(
        r#"
        SELECT role::text
        FROM developer_role_assignments
        WHERE tenant_id = $1
          AND principal_id = $2
          AND revoked_at IS NULL
        ORDER BY role::text
        "#,
    )
    .bind(tenant_id)
    .bind(principal_id)
    .fetch_all(db)
    .await?;

    rows.into_iter()
        .map(|role| parse_developer_role(&role))
        .collect()
}

pub fn permissions_for_roles(roles: &[DeveloperRole]) -> Vec<DeveloperPermission> {
    let mut permissions = HashSet::new();
    for role in roles {
        permissions.extend(permissions_for_role(*role));
    }
    let mut permissions = permissions.into_iter().collect::<Vec<_>>();
    permissions.sort_by_key(|permission| format!("{permission:?}"));
    permissions
}

pub fn parse_developer_role(role: &str) -> Result<DeveloperRole, AppError> {
    match role {
        "developer_admin" => Ok(DeveloperRole::DeveloperAdmin),
        "app_manager" => Ok(DeveloperRole::AppManager),
        "webhook_manager" => Ok(DeveloperRole::WebhookManager),
        "log_viewer" => Ok(DeveloperRole::LogViewer),
        "integration_tester" => Ok(DeveloperRole::IntegrationTester),
        "docs_viewer" => Ok(DeveloperRole::DocsViewer),
        _ => Err(AppError::internal(
            "developer_role_unknown",
            format!("Unknown developer role: {role}."),
        )),
    }
}
```

- [ ] **Step 2: Add route handler**

Create `apps/account-service/src/identity.domains.developer.routes.rs`:

```rust
use axum::{Extension, Json, Router, extract::State, routing::get};
use nvbes_core::http::error::ErrorEnvelope;

use crate::http::middleware::jwt::AuthContext;
use crate::{app::AppState, http::error::AppError};

pub fn router(state: &AppState) -> Router<AppState> {
    Router::new().route(
        "/developer/me",
        get(me).layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::http::middleware::jwt::jwt_auth_middleware,
        )),
    )
}

#[utoipa::path(
    get,
    path = "/developer/me",
    tag = "developer",
    responses(
        (status = 200, description = "Current developer portal access", body = crate::domains::developer::types::DeveloperMeResponse),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 403, description = "Forbidden", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub async fn me(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<crate::domains::developer::types::DeveloperMeResponse>, AppError> {
    let tenant_id = auth.tenant_id.ok_or_else(|| {
        AppError::forbidden(
            "tenant_context_required",
            "A tenant context is required before using the developer portal.",
        )
    })?;
    let roles =
        crate::domains::developer::rbac_db::load_developer_roles(&state.db, tenant_id, auth.user_id)
            .await?;
    let permissions = crate::domains::developer::rbac_db::permissions_for_roles(&roles);

    Ok(Json(crate::domains::developer::types::DeveloperMeResponse {
        tenant_id: tenant_id.to_string(),
        roles,
        permissions,
    }))
}
```

- [ ] **Step 3: Wire the module**

Modify `apps/account-service/src/identity.domains.developer.mod.rs`:

```rust
use axum::Router;

use crate::app::AppState;

#[path = "identity.domains.developer.types.rs"]
pub mod types;
#[path = "identity.domains.developer.rbac.rs"]
pub mod rbac;
#[path = "identity.domains.developer.rbac.db.rs"]
pub mod rbac_db;
#[path = "identity.domains.developer.routes.rs"]
pub mod routes;

#[cfg(test)]
#[path = "identity.domains.developer.tests.rs"]
mod tests;

pub fn router(state: &AppState) -> Router<AppState> {
    routes::router(state)
}
```

- [ ] **Step 4: Add OpenAPI tag and path**

Modify `apps/account-service/src/identity.http.openapi.rs`:

```rust
(name = "developer", description = "Developer portal, apps, webhooks, logs, and integration tools"),
```

Add this path to the `paths(...)` list:

```rust
crate::domains::developer::routes::me,
```

- [ ] **Step 5: Verify Rust compile**

Run:

```bash
rtk cargo check -p nvbes-account-service
```

Expected: pass.

- [ ] **Step 6: Commit**

```bash
rtk git add apps/account-service/src/identity.domains.developer.*.rs apps/account-service/src/identity.http.openapi.rs
rtk git commit -m "feat(identity): expose developer portal access"
```

---

## Task 4: Implement Developer Apps Facade

**Files:**

- Create: `apps/account-service/src/identity.domains.developer.apps.routes.rs`
- Create: `apps/account-service/src/identity.domains.developer.apps.service.rs`
- Modify: `apps/account-service/src/identity.domains.developer.mod.rs`
- Modify: `apps/account-service/src/identity.domains.developer.types.rs`
- Modify: `apps/account-service/src/identity.http.openapi.rs`

- [ ] **Step 1: Add app DTOs**

Append to `apps/account-service/src/identity.domains.developer.types.rs`:

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, ToSchema)]
pub struct DeveloperAppView {
    pub id: Uuid,
    pub client_id: String,
    pub name: String,
    pub redirect_uris: Vec<String>,
    pub client_type: String,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DeveloperAppsResponse {
    pub apps: Vec<DeveloperAppView>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateDeveloperAppRequest {
    pub name: String,
    pub redirect_uris: Vec<String>,
    pub allowed_scopes: Vec<String>,
    pub allowed_audiences: Vec<String>,
    pub allowed_resources: Vec<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CreateDeveloperAppResponse {
    pub app: DeveloperAppView,
    pub client_secret: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateDeveloperRedirectsRequest {
    pub redirect_uris: Vec<String>,
}
```

- [ ] **Step 2: Add permission helper**

Append to `apps/account-service/src/identity.domains.developer.rbac.rs`:

```rust
pub fn has_permission(permissions: &[DeveloperPermission], required: DeveloperPermission) -> bool {
    permissions.contains(&required)
}
```

- [ ] **Step 3: Add service facade**

Create `apps/account-service/src/identity.domains.developer.apps.service.rs`:

```rust
use sqlx::PgPool;

use crate::domains::developer::types::{
    CreateDeveloperAppRequest, CreateDeveloperAppResponse, DeveloperAppView, DeveloperAppsResponse,
    UpdateDeveloperRedirectsRequest,
};
use crate::domains::oauth::service::types::CreateOAuthClientInput;
use crate::http::error::AppError;

pub async fn list_apps(
    db: &PgPool,
    auth: &(impl crate::domains::oauth::logic::OAuthManagementAuth
        + crate::domains::authz::TenantManagementAuth),
) -> Result<DeveloperAppsResponse, AppError> {
    let result = crate::domains::oauth::clients::list_clients(db, auth).await?;
    Ok(DeveloperAppsResponse {
        apps: result.clients.into_iter().map(DeveloperAppView::from).collect(),
    })
}

pub async fn create_app(
    db: &PgPool,
    auth: &(impl crate::domains::oauth::logic::OAuthManagementAuth
        + crate::domains::authz::TenantManagementAuth),
    request: CreateDeveloperAppRequest,
) -> Result<CreateDeveloperAppResponse, AppError> {
    let result = crate::domains::oauth::clients::create_client(
        db,
        auth,
        CreateOAuthClientInput {
            name: request.name,
            redirect_uris: request.redirect_uris,
            allowed_scopes: request.allowed_scopes,
            allowed_audiences: request.allowed_audiences,
            allowed_resources: request.allowed_resources,
            owner_scope_type: Some("tenant".to_string()),
            owner_scope_id: crate::domains::oauth::logic::OAuthManagementAuth::tenant_id(auth)
                .ok_or_else(|| AppError::forbidden("tenant_context_required", "A tenant context is required."))?,
            client_type: Some("confidential".to_string()),
            required_acr: Some("aal1".to_string()),
            client_assertion_public_key_jwk: None,
            client_assertion_required: Some(false),
            service_account_role: None,
        },
    )
    .await?;

    Ok(CreateDeveloperAppResponse {
        app: DeveloperAppView::from(result.client),
        client_secret: result.client_secret,
    })
}

pub async fn update_redirects(
    db: &PgPool,
    client_id: &str,
    request: UpdateDeveloperRedirectsRequest,
) -> Result<DeveloperAppView, AppError> {
    if request.redirect_uris.is_empty() {
        return Err(AppError::bad_request(
            "validation_failed",
            "At least one redirect URI is required.",
        ));
    }

    let row = sqlx::query_as::<_, crate::domains::oauth::service::types::OAuthClientView>(
        r#"
        UPDATE oauth_clients
        SET redirect_uris = $2
        WHERE client_id = $1
          AND revoked_at IS NULL
        RETURNING id, client_id, name, redirect_uris, created_at, last_used_at, tenant_id,
          owner_scope_type::text AS owner_scope_type, owner_scope_id, client_type::text AS client_type,
          client_assertion_required, client_assertion_public_key_jwk IS NOT NULL AS client_assertion_public_key_configured,
          service_account_principal_id, service_account_workspace_id, service_account_role::text AS service_account_role
        "#,
    )
    .bind(client_id)
    .bind(&request.redirect_uris)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::not_found("oauth_client_not_found", "OAuth client not found."))?;

    Ok(DeveloperAppView::from(row))
}

impl From<crate::domains::oauth::service::types::OAuthClientView> for DeveloperAppView {
    fn from(client: crate::domains::oauth::service::types::OAuthClientView) -> Self {
        Self {
            id: client.id,
            client_id: client.client_id,
            name: client.name,
            redirect_uris: client.redirect_uris,
            client_type: client.client_type,
            created_at: client.created_at,
            last_used_at: client.last_used_at,
        }
    }
}
```

- [ ] **Step 4: Add routes**

Create `apps/account-service/src/identity.domains.developer.apps.routes.rs` with handlers for list, create, get, update redirects, and revoke. Each handler must load permissions with `rbac_db::permissions_for_roles`, enforce the required permission, then call the service. Use required permissions:

```rust
DeveloperPermission::AppsRead
DeveloperPermission::AppsCreate
DeveloperPermission::AppsUpdateRedirects
DeveloperPermission::AppsRevoke
```

Use the same `jwt_auth_middleware` pattern as `identity.domains.developer.routes.rs`.

- [ ] **Step 5: Wire app routes**

Modify `apps/account-service/src/identity.domains.developer.mod.rs`:

```rust
#[path = "identity.domains.developer.apps.routes.rs"]
pub mod apps_routes;
#[path = "identity.domains.developer.apps.service.rs"]
pub mod apps_service;
```

Merge `apps_routes::router(state)` inside `router`.

- [ ] **Step 6: Add OpenAPI paths**

Add developer app route functions to `IdentityApiDoc` paths:

```rust
crate::domains::developer::apps_routes::list_apps,
crate::domains::developer::apps_routes::create_app,
crate::domains::developer::apps_routes::get_app,
crate::domains::developer::apps_routes::update_redirects,
crate::domains::developer::apps_routes::revoke_app,
```

- [ ] **Step 7: Verify backend**

Run:

```bash
rtk cargo check -p nvbes-account-service
```

Expected: pass. If `CreateOAuthClientInput` fields differ because of concurrent OAuth work, adapt only the constructor in `apps.service.rs` to the current type and keep the route contract unchanged.

- [ ] **Step 8: Commit**

```bash
rtk git add apps/account-service/src/identity.domains.developer.*.rs apps/account-service/src/identity.http.openapi.rs
rtk git commit -m "feat(identity): add developer app management facade"
```

---

## Task 5: Implement Developer Webhooks

**Files:**

- Create: `apps/account-service/src/identity.domains.developer.webhooks.service.rs`
- Create: `apps/account-service/src/identity.domains.developer.webhooks.routes.rs`
- Modify: `apps/account-service/src/identity.domains.developer.mod.rs`
- Modify: `apps/account-service/src/identity.domains.developer.types.rs`
- Modify: `apps/account-service/src/identity.http.openapi.rs`

- [ ] **Step 1: Add webhook DTOs**

Append to `apps/account-service/src/identity.domains.developer.types.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DeveloperWebhookEventType {
    UserCreated,
    LoginFailed,
    SessionRevoked,
    ClientCreated,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DeveloperWebhookEndpointView {
    pub id: Uuid,
    pub name: String,
    pub url: String,
    pub status: String,
    pub events: Vec<DeveloperWebhookEventType>,
    pub signing_secret_last4: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateDeveloperWebhookEndpointRequest {
    pub name: String,
    pub url: String,
    pub events: Vec<DeveloperWebhookEventType>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CreateDeveloperWebhookEndpointResponse {
    pub endpoint: DeveloperWebhookEndpointView,
    pub signing_secret: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DeveloperWebhooksResponse {
    pub endpoints: Vec<DeveloperWebhookEndpointView>,
}
```

- [ ] **Step 2: Add webhook service**

Create `apps/account-service/src/identity.domains.developer.webhooks.service.rs` with these public functions:

```rust
pub async fn list_endpoints(
    db: &sqlx::PgPool,
    tenant_id: uuid::Uuid,
) -> Result<crate::domains::developer::types::DeveloperWebhooksResponse, crate::http::error::AppError>;

pub async fn create_endpoint(
    db: &sqlx::PgPool,
    tenant_id: uuid::Uuid,
    created_by: uuid::Uuid,
    request: crate::domains::developer::types::CreateDeveloperWebhookEndpointRequest,
) -> Result<crate::domains::developer::types::CreateDeveloperWebhookEndpointResponse, crate::http::error::AppError>;

pub async fn delete_endpoint(
    db: &sqlx::PgPool,
    tenant_id: uuid::Uuid,
    endpoint_id: uuid::Uuid,
) -> Result<(), crate::http::error::AppError>;
```

Generate signing secrets with:

```rust
let signing_secret = format!("whsec_{}", uuid::Uuid::new_v4().simple());
let signing_secret_last4 = signing_secret
    .chars()
    .rev()
    .take(4)
    .collect::<Vec<_>>()
    .into_iter()
    .rev()
    .collect::<String>();
```

For V1, store `signing_secret_ciphertext` as the secret string only if no encryption helper exists in the current codebase. Before merging, search for existing encryption helpers with:

```bash
rtk rg -n "encrypt|ciphertext|secret_ciphertext|decrypt" apps libs/rust
```

If a repo helper exists, use it and keep the route response unchanged.

- [ ] **Step 3: Add webhook routes**

Create `apps/account-service/src/identity.domains.developer.webhooks.routes.rs` with:

- `GET /developer/webhooks` requiring `DeveloperPermission::WebhooksRead`
- `POST /developer/webhooks` requiring `DeveloperPermission::WebhooksManage`
- `DELETE /developer/webhooks/{endpointId}` requiring `DeveloperPermission::WebhooksManage`

Use `AuthContext.tenant_id` and `AuthContext.user_id` as tenant and actor context.

- [ ] **Step 4: Wire routes and OpenAPI**

Modify `developer.mod.rs` to declare and merge webhook routes.

Modify `identity.http.openapi.rs` to add:

```rust
crate::domains::developer::webhooks_routes::list_webhooks,
crate::domains::developer::webhooks_routes::create_webhook,
crate::domains::developer::webhooks_routes::delete_webhook,
```

- [ ] **Step 5: Verify backend**

Run:

```bash
rtk cargo check -p nvbes-account-service
```

Expected: pass.

- [ ] **Step 6: Commit**

```bash
rtk git add apps/account-service/src/identity.domains.developer.*.rs apps/account-service/src/identity.http.openapi.rs
rtk git commit -m "feat(identity): add developer webhooks"
```

---

## Task 6: Implement Token Inspector, OAuth Playground, And Logs Routes

**Files:**

- Create: `apps/account-service/src/identity.domains.developer.tokens.routes.rs`
- Create: `apps/account-service/src/identity.domains.developer.logs.routes.rs`
- Modify: `apps/account-service/src/identity.domains.developer.mod.rs`
- Modify: `apps/account-service/src/identity.domains.developer.types.rs`
- Modify: `apps/account-service/src/identity.http.openapi.rs`

- [ ] **Step 1: Add DTOs**

Append to `identity.domains.developer.types.rs`:

```rust
#[derive(Debug, Deserialize, ToSchema)]
pub struct InspectDeveloperTokenRequest {
    pub token: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct InspectDeveloperTokenResponse {
    pub active: bool,
    pub subject: Option<String>,
    pub client_id: Option<String>,
    pub tenant_id: Option<String>,
    pub scopes: Vec<String>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct OAuthPlaygroundExchangeRequest {
    pub client_id: String,
    pub code: String,
    pub redirect_uri: String,
    pub code_verifier: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct OAuthPlaygroundExchangeResponse {
    pub token_type: String,
    pub expires_in: i64,
    pub scope: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DeveloperLogEntry {
    pub id: String,
    pub event_type: String,
    pub user_id: Option<String>,
    pub client_id: Option<String>,
    pub tenant_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DeveloperLogsResponse {
    pub logs: Vec<DeveloperLogEntry>,
}
```

- [ ] **Step 2: Implement token routes**

Create `apps/account-service/src/identity.domains.developer.tokens.routes.rs` with:

- `POST /developer/tokens/inspect` requiring `DeveloperPermission::TokensInspect`
- `POST /developer/oauth/playground/exchange` requiring `DeveloperPermission::OAuthPlayground`

Token inspector must call existing JWT validation/introspection helpers when available. It must never write the raw token to logs, audit rows, or error messages. For the first compiling implementation, return inactive for validation failures:

```rust
Ok(Json(InspectDeveloperTokenResponse {
    active: false,
    subject: None,
    client_id: None,
    tenant_id: None,
    scopes: Vec::new(),
    expires_at: None,
}))
```

- [ ] **Step 3: Implement logs route**

Create `apps/account-service/src/identity.domains.developer.logs.routes.rs` with `GET /developer/logs` requiring `DeveloperPermission::LogsRead`. Query params:

```rust
#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct DeveloperLogsQuery {
    pub user_id: Option<String>,
    pub client_id: Option<String>,
    pub tenant_id: Option<String>,
    pub event_type: Option<String>,
}
```

Use existing security/audit tables if present. If no shared event table supports all filters, return an empty array with a valid response shape and add a follow-up task inside this same implementation branch before merging to connect the final source table. Do not expose cross-tenant logs.

- [ ] **Step 4: Wire routes and OpenAPI**

Declare and merge tokens/logs routes in `developer.mod.rs`. Add paths:

```rust
crate::domains::developer::tokens_routes::inspect_token,
crate::domains::developer::tokens_routes::exchange_playground_code,
crate::domains::developer::logs_routes::list_logs,
```

- [ ] **Step 5: Verify backend**

Run:

```bash
rtk cargo check -p nvbes-account-service
```

Expected: pass.

- [ ] **Step 6: Commit**

```bash
rtk git add apps/account-service/src/identity.domains.developer.*.rs apps/account-service/src/identity.http.openapi.rs
rtk git commit -m "feat(identity): add developer integration tools"
```

---

## Task 7: Generate OpenAPI And SDK Types

**Files:**

- Modify: `apps/account-service/openapi.json`
- Modify: `libs/ts/identity-sdk-core/openapi.json`
- Modify: `libs/ts/identity-sdk-core/src/types.gen.ts`
- Create: `docs/api/identity-developer-portal.md`

- [ ] **Step 1: Generate OpenAPI**

Run:

```bash
rtk pnpm generate:openapi
```

Expected: `apps/account-service/openapi.json` and `libs/ts/identity-sdk-core/openapi.json` include `/developer/me`, `/developer/apps`, `/developer/webhooks`, `/developer/tokens/inspect`, `/developer/oauth/playground/exchange`, and `/developer/logs`.

- [ ] **Step 2: Generate TypeScript OpenAPI types**

Run:

```bash
rtk pnpm --dir libs/ts/identity-sdk-core generate:ts
```

Expected: `libs/ts/identity-sdk-core/src/types.gen.ts` includes developer schemas and paths.

- [ ] **Step 3: Add API reference docs**

Create `docs/api/identity-developer-portal.md`:

```markdown
# Identity Developer Portal API

The developer portal API is served by `account-service` under `/developer`.

## Permissions

| Permission | Purpose |
| --- | --- |
| `developer.apps.read` | Read OAuth apps exposed in the developer portal. |
| `developer.apps.create` | Create OAuth apps. |
| `developer.apps.update_redirects` | Update app redirect URIs. |
| `developer.apps.revoke` | Revoke OAuth apps. |
| `developer.webhooks.read` | Read webhook endpoints and delivery status. |
| `developer.webhooks.manage` | Create, update, and delete webhook endpoints. |
| `developer.logs.read` | Read tenant-scoped developer logs. |
| `developer.tokens.inspect` | Inspect tokens without logging raw token bodies. |
| `developer.oauth.playground` | Exchange OAuth playground authorization codes. |
| `developer.rbac.manage` | Assign and revoke developer roles. |
| `developer.docs.read` | Access connected docs-only portal views. |

## Webhook Events

- `user.created`
- `login.failed`
- `session.revoked`
- `client.created`

Webhook deliveries are signed. The signing secret is displayed only when created or rotated.
```

- [ ] **Step 4: Verify generated files are stable**

Run:

```bash
rtk git diff -- apps/account-service/openapi.json libs/ts/identity-sdk-core/openapi.json libs/ts/identity-sdk-core/src/types.gen.ts docs/api/identity-developer-portal.md
```

Expected: only developer API additions and generated type changes.

- [ ] **Step 5: Commit**

```bash
rtk git add apps/account-service/openapi.json libs/ts/identity-sdk-core/openapi.json libs/ts/identity-sdk-core/src/types.gen.ts docs/api/identity-developer-portal.md
rtk git commit -m "docs(api): expose developer portal contract"
```

---

## Task 8: Build Developer Frontend API Layer And Auth Shell

**Files:**

- Create: `apps/console-web/src/developer.schemas.ts`
- Create: `apps/console-web/src/developer.api.ts`
- Create: `apps/console-web/src/layouts/DeveloperPublicLayout.tsx`
- Create: `apps/console-web/src/layouts/DeveloperPortalLayout.tsx`
- Modify: `apps/console-web/src/developer.router.tsx`

- [ ] **Step 1: Add Zod schemas**

Create `apps/console-web/src/developer.schemas.ts`:

```ts
import { z } from 'zod';

export const DeveloperPermissionSchema = z.enum([
  'developer.apps.read',
  'developer.apps.create',
  'developer.apps.update_redirects',
  'developer.apps.revoke',
  'developer.webhooks.read',
  'developer.webhooks.manage',
  'developer.logs.read',
  'developer.tokens.inspect',
  'developer.oauth.playground',
  'developer.rbac.manage',
  'developer.docs.read',
]);

export const DeveloperMeSchema = z.object({
  tenant_id: z.string(),
  roles: z.array(z.string()),
  permissions: z.array(DeveloperPermissionSchema),
});

export const DeveloperAppSchema = z.object({
  id: z.string(),
  client_id: z.string(),
  name: z.string(),
  redirect_uris: z.array(z.string()),
  client_type: z.string(),
  created_at: z.string(),
  last_used_at: z.string().nullable().optional(),
});

export const DeveloperAppsResponseSchema = z.object({
  apps: z.array(DeveloperAppSchema),
});

export const DeveloperWebhooksResponseSchema = z.object({
  endpoints: z.array(
    z.object({
      id: z.string(),
      name: z.string(),
      url: z.string(),
      status: z.string(),
      events: z.array(z.string()),
      signing_secret_last4: z.string(),
      created_at: z.string(),
    }),
  ),
});

export type DeveloperPermission = z.infer<typeof DeveloperPermissionSchema>;
export type DeveloperMe = z.infer<typeof DeveloperMeSchema>;
export type DeveloperApp = z.infer<typeof DeveloperAppSchema>;
```

- [ ] **Step 2: Add API functions**

Create `apps/console-web/src/developer.api.ts`:

```ts
import { identityHttpClient } from '@nvbes/identity-client';
import {
  DeveloperAppsResponseSchema,
  DeveloperMeSchema,
  DeveloperWebhooksResponseSchema,
  type DeveloperMe,
} from './developer.schemas';

export function getDeveloperMe(options?: { signal?: AbortSignal }) {
  return identityHttpClient.get('/developer/me', DeveloperMeSchema, options);
}

export function listDeveloperApps(options?: { signal?: AbortSignal }) {
  return identityHttpClient
    .get('/developer/apps', DeveloperAppsResponseSchema, options)
    .then((response) => response.apps);
}

export function listDeveloperWebhooks(options?: { signal?: AbortSignal }) {
  return identityHttpClient
    .get('/developer/webhooks', DeveloperWebhooksResponseSchema, options)
    .then((response) => response.endpoints);
}

export function hasDeveloperPermission(
  me: DeveloperMe | undefined,
  permission: DeveloperMe['permissions'][number],
): boolean {
  return Boolean(me?.permissions.includes(permission));
}
```

- [ ] **Step 3: Add portal layout**

Create `apps/console-web/src/layouts/DeveloperPortalLayout.tsx`:

```tsx
import { useQuery } from '@tanstack/react-query';
import { Link, Outlet } from '@tanstack/react-router';
import { AppWindow, FileJson, KeyRound, ListFilter, RadioTower, Webhook } from 'lucide-react';
import { getDeveloperMe } from '@/developer.api';

const navItems = [
  { to: '/portal/apps', label: 'Apps', icon: AppWindow },
  { to: '/portal/tokens/inspect', label: 'Tokens', icon: KeyRound },
  { to: '/portal/oauth/playground', label: 'OAuth', icon: RadioTower },
  { to: '/portal/logs', label: 'Logs', icon: ListFilter },
  { to: '/portal/webhooks', label: 'Webhooks', icon: Webhook },
  { to: '/api-reference', label: 'OpenAPI', icon: FileJson },
] as const;

export function DeveloperPortalLayout() {
  const meQuery = useQuery({
    queryKey: ['developer', 'me'],
    queryFn: ({ signal }) => getDeveloperMe({ signal }),
    retry: false,
  });

  if (meQuery.isLoading) {
    return <main className="min-h-screen bg-background p-6 text-sm text-muted-foreground">Loading portal...</main>;
  }

  if (meQuery.isError) {
    return (
      <main className="min-h-screen bg-background p-6">
        <div className="rounded-md border border-border bg-card p-6">
          <h1 className="text-lg font-semibold">Developer access required</h1>
          <p className="mt-2 text-sm text-muted-foreground">
            Sign in with a tenant account that has developer portal permissions.
          </p>
        </div>
      </main>
    );
  }

  return (
    <div className="grid min-h-screen grid-cols-1 bg-background text-foreground md:grid-cols-[240px_1fr]">
      <aside className="border-b border-border bg-card md:border-b-0 md:border-r">
        <div className="border-b border-border px-4 py-4">
          <Link to="/" className="font-heading text-base font-semibold">
            nvbes Developers
          </Link>
          <p className="mt-1 text-xs text-muted-foreground">Tenant {meQuery.data.tenant_id}</p>
        </div>
        <nav className="flex gap-1 overflow-x-auto px-2 py-3 md:flex-col">
          {navItems.map((item) => (
            <Link
              key={item.to}
              to={item.to}
              className="flex items-center gap-2 rounded-md px-3 py-2 text-sm text-muted-foreground hover:bg-muted hover:text-foreground"
            >
              <item.icon className="size-4" />
              {item.label}
            </Link>
          ))}
        </nav>
      </aside>
      <main className="min-w-0 overflow-auto">
        <div className="mx-auto w-full max-w-7xl px-4 py-6 md:px-8">
          <Outlet />
        </div>
      </main>
    </div>
  );
}
```

- [ ] **Step 4: Add public layout**

Create `apps/console-web/src/layouts/DeveloperPublicLayout.tsx`:

```tsx
import { Link, Outlet } from '@tanstack/react-router';

export function DeveloperPublicLayout() {
  return (
    <main className="min-h-screen bg-background text-foreground">
      <header className="border-b border-border bg-card">
        <nav className="mx-auto flex h-14 max-w-7xl items-center justify-between px-4 md:px-8">
          <Link to="/" className="font-heading text-base font-semibold">
            nvbes Developers
          </Link>
          <div className="flex items-center gap-4 text-sm">
            <Link to="/quickstarts/react">Quickstarts</Link>
            <Link to="/api-reference">API Reference</Link>
            <Link to="/portal/apps" className="font-medium text-primary">
              Portal
            </Link>
          </div>
        </nav>
      </header>
      <Outlet />
    </main>
  );
}
```

- [ ] **Step 5: Run frontend checks**

Run:

```bash
rtk pnpm exec nx run console-web:typecheck
rtk pnpm exec nx run console-web:lint
```

Expected: pass.

- [ ] **Step 6: Commit**

```bash
rtk git add apps/console-web/src
rtk git commit -m "feat(developer): add portal API shell"
```

---

## Task 9: Build Portal Screens

**Files:**

- Create/modify page files under `apps/console-web/src/pages/`
- Modify: `apps/console-web/src/developer.router.tsx`

- [ ] **Step 1: Create page components**

Create these page files with focused components:

```text
apps/console-web/src/pages/PortalOverviewPage.tsx
apps/console-web/src/pages/PortalAppsPage.tsx
apps/console-web/src/pages/PortalAppDetailPage.tsx
apps/console-web/src/pages/PortalTokenInspectorPage.tsx
apps/console-web/src/pages/PortalOAuthPlaygroundPage.tsx
apps/console-web/src/pages/PortalLogsPage.tsx
apps/console-web/src/pages/PortalWebhooksPage.tsx
apps/console-web/src/pages/PortalRolesPage.tsx
```

Use this empty-state pattern in every data page:

```tsx
export function EmptyState({ title, body }: { title: string; body: string }) {
  return (
    <div className="rounded-md border border-dashed border-border bg-card p-8">
      <h2 className="text-base font-semibold">{title}</h2>
      <p className="mt-2 max-w-xl text-sm text-muted-foreground">{body}</p>
    </div>
  );
}
```

- [ ] **Step 2: Implement apps page**

`PortalAppsPage.tsx` must call `listDeveloperApps`, render a table with `name`, `client_id`, `client_type`, `redirect_uris.length`, and include a copy button:

```tsx
async function copyClientId(clientId: string) {
  await navigator.clipboard.writeText(clientId);
}
```

The create app form must submit to a typed `createDeveloperApp` API function and display the returned `client_secret` in a one-time visible panel.

- [ ] **Step 3: Implement token inspector**

`PortalTokenInspectorPage.tsx` must parse JWTs client-side without trusting them:

```ts
function decodeJwtSegment(segment: string): unknown {
  const normalized = segment.replace(/-/g, '+').replace(/_/g, '/');
  const padded = normalized.padEnd(Math.ceil(normalized.length / 4) * 4, '=');
  return JSON.parse(window.atob(padded)) as unknown;
}
```

Render parse errors as form feedback. Submit to `/developer/tokens/inspect` only when the input is non-empty.

- [ ] **Step 4: Implement OAuth playground**

Generate PKCE with Web Crypto:

```ts
async function sha256Base64Url(value: string): Promise<string> {
  const bytes = new TextEncoder().encode(value);
  const digest = await crypto.subtle.digest('SHA-256', bytes);
  return btoa(String.fromCharCode(...new Uint8Array(digest)))
    .replace(/\+/g, '-')
    .replace(/\//g, '_')
    .replace(/=+$/g, '');
}
```

The page must show authorize URL, `state`, `nonce`, code verifier, and code challenge. The exchange form posts to the backend route from Task 6.

- [ ] **Step 5: Implement logs and webhooks pages**

`PortalLogsPage.tsx` must render filters for `user_id`, `client_id`, `tenant_id`, `event_type`, and date range. `PortalWebhooksPage.tsx` must render endpoint list, creation form, event checkboxes, and one-time signing secret display.

- [ ] **Step 6: Wire routes**

Replace the initial `developer.router.tsx` route tree with:

```tsx
const publicRoute = createRoute({
  getParentRoute: () => rootRoute,
  id: 'public',
  component: DeveloperPublicLayout,
});

const portalRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/portal',
  component: DeveloperPortalLayout,
});
```

Add public and portal child routes for every route listed in the spec.

- [ ] **Step 7: Verify frontend**

Run:

```bash
rtk pnpm exec nx run console-web:typecheck
rtk pnpm exec nx run console-web:lint
```

Expected: pass.

- [ ] **Step 8: Commit**

```bash
rtk git add apps/console-web/src
rtk git commit -m "feat(developer): build portal screens"
```

---

## Task 10: Add Quickstarts And API Reference Pages

**Files:**

- Create: `apps/console-web/src/developer.quickstarts.ts`
- Create: `apps/console-web/src/pages/DeveloperHomePage.tsx`
- Create: `apps/console-web/src/pages/QuickstartPage.tsx`
- Create: `apps/console-web/src/pages/ApiReferencePage.tsx`
- Modify: `apps/console-web/src/developer.router.tsx`
- Modify: `libs/ts/identity-sdk/README.md`

- [ ] **Step 1: Add quickstart content**

Create `apps/console-web/src/developer.quickstarts.ts`:

```ts
export const quickstarts = {
  react: {
    title: 'React quickstart',
    command: 'pnpm add @nvbes/identity-sdk-web',
    code: `import { createIdentityClient } from '@nvbes/identity-sdk-web';

export const identity = createIdentityClient({
  issuer: import.meta.env.VITE_NVBES_ACCOUNT_ISSUER,
  clientId: import.meta.env.VITE_NVBES_CLIENT_ID,
  redirectUri: window.location.origin + '/auth/callback',
});`,
  },
  'rust-axum': {
    title: 'Rust Axum quickstart',
    command: 'cargo add nvbes-identity-sdk-backend',
    code: `use axum::{routing::get, Router};

async fn me() -> &'static str {
    "validated nvbes Identity session"
}

pub fn router() -> Router {
    Router::new().route("/me", get(me))
}`,
  },
  node: {
    title: 'Node quickstart',
    command: 'pnpm add @nvbes/identity-sdk',
    code: `import { createIdentityClient } from '@nvbes/identity-sdk';

const identity = createIdentityClient({
  baseUrl: process.env.NVBES_ACCOUNT_SERVICE_URL,
});`,
  },
  curl: {
    title: 'curl quickstart',
    command: 'curl https://account.nvbes.fr/.well-known/openid-configuration',
    code: `curl -s https://account.nvbes.fr/.well-known/openid-configuration | jq .issuer`,
  },
} as const;
```

- [ ] **Step 2: Add docs pages**

`QuickstartPage.tsx` must read the quickstart key from route params and render `title`, `command`, and `code` in copyable code blocks.

`ApiReferencePage.tsx` must fetch `/api/openapi.json` and render:

- API title and version.
- Number of paths.
- A filtered list of paths starting with `/developer`.
- A link to `/docs` for Swagger UI.

- [ ] **Step 3: Update SDK README**

Append to `libs/ts/identity-sdk/README.md`:

```markdown
## Developer Portal

The nvbes Developer Portal includes quickstarts for React, Rust Axum, Node, and curl. It also links to the generated Identity OpenAPI document and developer portal routes under `/developer`.
```

- [ ] **Step 4: Verify frontend**

Run:

```bash
rtk pnpm exec nx run console-web:typecheck
rtk pnpm exec nx run console-web:lint
```

Expected: pass.

- [ ] **Step 5: Commit**

```bash
rtk git add apps/console-web/src libs/ts/identity-sdk/README.md
rtk git commit -m "feat(developer): add quickstarts and api reference"
```

---

## Task 11: Final Verification And Browser QA

**Files:**

- Modify only files required by check failures.

- [ ] **Step 1: Run backend verification**

Run:

```bash
rtk cargo check --workspace
```

Expected: pass.

- [ ] **Step 2: Run frontend verification**

Run:

```bash
rtk pnpm exec nx run console-web:typecheck
rtk pnpm exec nx run console-web:lint
rtk pnpm check:structure
rtk pnpm check:nx-boundaries
```

Expected: pass.

- [ ] **Step 3: Run OpenAPI verification**

Run:

```bash
rtk pnpm generate:openapi
rtk git diff --exit-code apps/account-service/openapi.json libs/ts/identity-sdk-core/openapi.json libs/ts/identity-sdk-core/src/types.gen.ts
```

Expected: no diff after generated files are committed.

- [ ] **Step 4: Start the dev server**

Run:

```bash
rtk pnpm dev:console-web
```

Expected: Vite starts on `http://localhost:5175`.

- [ ] **Step 5: Browser QA**

Open `http://localhost:5175` in the in-app browser. Verify:

- Public homepage first viewport clearly says `nvbes Developers`.
- Quickstart pages render React, Rust Axum, Node, and curl content.
- `/api-reference` renders OpenAPI metadata or a clear API connection error.
- `/portal/apps` renders access denied when unauthenticated.
- Mobile viewport has no overlapping text or broken navigation.

- [ ] **Step 6: Commit final fixes**

```bash
rtk git add apps/console-web apps/account-service libs/ts/identity-sdk-core libs/ts/identity-sdk docs/api package.json scripts
rtk git commit -m "chore(developer): verify developer portal"
```

---

## Self-Review

Spec coverage:

- Dedicated `apps/console-web`: covered by Tasks 1, 8, 9, and 10.
- Developer RBAC roles and fine permissions: covered by Tasks 2 and 3.
- Dashboard app creation, redirects, client ID copy: covered by Tasks 4 and 9.
- OpenAPI and generated SDK types: covered by Task 7.
- Quickstarts React, Rust Axum, Node, curl: covered by Task 10.
- Token inspector: covered by Tasks 6 and 9.
- OAuth playground: covered by Tasks 6 and 9.
- Logs filtered by user/client/tenant: covered by Tasks 6 and 9.
- Webhooks for `user.created`, `login.failed`, `session.revoked`, `client.created`: covered by Tasks 2, 5, and 9.
- Browser verification: covered by Task 11.

Type consistency:

- Backend uses `DeveloperRole`, `DeveloperPermission`, `DeveloperMeResponse`, `DeveloperAppView`, and webhook DTOs from `identity.domains.developer.types.rs`.
- Frontend uses matching Zod schemas in `developer.schemas.ts`.
- Route names match the spec and the OpenAPI task.

Scope:

- The plan builds the V1 production-connected vertical in one implementation branch with frequent commits.
- Custom tenant roles, ABAC policy builder, and a separate `developer-api` service remain outside this plan.
