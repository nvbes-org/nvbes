# Identity Developer Console V0 Implementation Plan

> **Status: inactive historical plan.** Do not restore `console-web` from this
> plan during the foundation V1. See the
> [active direction](../../product/nvbes-product-strategy.md).

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a dedicated `console-web` V0 for nvbes Identity developers with OAuth app governance, scope registry, service accounts, secret rotation, webhooks, logs, token debugging, sandbox status, and integration health checks.

**Architecture:** Add `apps/console-web` as a standalone React/Vite app and add `identity.domains.developer.*` in `account-service` as a tenant-scoped facade. The developer domain owns product-specific state and permissions while delegating OAuth, service-account, token, and audit behavior to existing Identity primitives.

**Tech Stack:** Rust, Axum, SQLx, PostgreSQL, utoipa, pnpm workspaces, Nx, React, TypeScript, Vite, TanStack Router, TanStack Query, Zod, `@nvbes/web-runtime`, `@nvbes/web-ui`, lucide-react.

---

## File Structure

Frontend app:

- Create `apps/console-web/package.json`: package scripts and dependencies.
- Create `apps/console-web/project.json`: Nx metadata.
- Create `apps/console-web/index.html`: Vite HTML entry.
- Create `apps/console-web/tsconfig.json`: TypeScript config.
- Create `apps/console-web/vite.config.ts`: Vite Plus config with API proxy.
- Create `apps/console-web/src/main.tsx`: React root.
- Create `apps/console-web/src/App.tsx`: providers and router mount.
- Create `apps/console-web/src/styles.css`: Tailwind theme tokens.
- Create `apps/console-web/src/developer.schemas.ts`: Zod schemas and inferred types.
- Create `apps/console-web/src/developer.api.ts`: typed API client functions.
- Create `apps/console-web/src/developer.fixtures.ts`: docs and deterministic empty-state copy.
- Create `apps/console-web/src/developer.router.tsx`: route tree.
- Create `apps/console-web/src/developer.permissions.ts`: route guard helpers.
- Create `apps/console-web/src/layouts/DeveloperShell.tsx`: authenticated console shell.
- Create `apps/console-web/src/pages/*.tsx`: Overview, OAuth apps, Marketplace, Scopes, Service accounts, Secrets, Webhooks, Logs, Token debugger, Sandbox, Health checks, Developer roles.
- Create `apps/console-web/src/__tests__/*.test.ts`: schema, permission, and workflow helper tests.

Backend domain:

- Create `apps/account-service/migrations/0008_developer_console.sql`: developer-specific persistence.
- Create `apps/account-service/src/identity.domains.developer.mod.rs`: module declarations.
- Create `apps/account-service/src/identity.domains.developer.types.rs`: DTOs and OpenAPI schemas.
- Create `apps/account-service/src/identity.domains.developer.rbac.rs`: pure permission mapping.
- Create `apps/account-service/src/identity.domains.developer.rbac.db.rs`: role assignment queries.
- Create `apps/account-service/src/identity.domains.developer.routes.rs`: router.
- Create `apps/account-service/src/identity.domains.developer.routes.context.rs`: context and overview handlers.
- Create `apps/account-service/src/identity.domains.developer.routes.oauth.rs`: OAuth client, marketplace, consent, scope routes.
- Create `apps/account-service/src/identity.domains.developer.routes.service_accounts.rs`: service-account and secret routes.
- Create `apps/account-service/src/identity.domains.developer.routes.webhooks.rs`: webhook and delivery replay routes.
- Create `apps/account-service/src/identity.domains.developer.routes.tools.rs`: logs, token debugger, sandbox, health routes.
- Create `apps/account-service/src/identity.domains.developer.service.rs`: orchestration helpers.
- Create `apps/account-service/src/identity.domains.developer.health.rs`: integration health check normalization.
- Create `apps/account-service/src/identity.domains.developer.token_debugger.rs`: token inspection summaries.
- Create `apps/account-service/src/identity.domains.developer.tests.rs`: focused unit tests.
- Modify `apps/account-service/src/identity.domains.mod.rs`: expose and merge developer router.
- Modify `apps/account-service/src/identity.http.openapi.rs`: include developer paths and schemas.

Root and docs:

- Modify `package.json`: include `console-web` in web scripts.
- Create `scripts/dev-console-web.sh`: local dev runner.
- Modify `scripts/dev-web.sh`: include developer app only if the existing script starts all web apps.
- Create `docs/api/identity-developer-console.md`: route and permission reference.
- Regenerate `libs/ts/identity-sdk-core/openapi.json` and `libs/ts/identity-sdk-core/src/types.gen.ts` after backend routes compile.

---

### Task 1: Scaffold `console-web`

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
- Create: `scripts/dev-console-web.sh`
- Modify: `package.json`

- [ ] **Step 1: Create the app package**

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
    "preview": "node ./node_modules/vite-plus/bin/vp preview",
    "typecheck": "tsgo -b --pretty false",
    "lint": "node ./node_modules/vite-plus/bin/vp lint",
    "format": "node ./node_modules/vite-plus/bin/vp fmt --write",
    "format:check": "node ./node_modules/vite-plus/bin/vp fmt --check"
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
    "vitest": "catalog:",
    "vite-plus": "catalog:"
  }
}
```

- [ ] **Step 2: Add project metadata and app config**

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

Create `apps/console-web/tsconfig.json`:

```json
{
  "extends": "../../tsconfig.base.json",
  "compilerOptions": {
    "tsBuildInfoFile": "./node_modules/.tmp/tsconfig.app.tsbuildinfo",
    "paths": {
      "@nvbes/http-client": ["../../libs/ts/http-client/src/index.ts"],
      "@nvbes/identity-client": ["../../libs/ts/identity-client/src/index.ts"],
      "@nvbes/identity-sdk-web": ["../../libs/ts/identity-sdk-web/src/index.ts"],
      "@nvbes/web-runtime": ["../../libs/ts/web-runtime/src/index.ts"],
      "@nvbes/web-ui": ["../../libs/ts/web-ui/src/index.ts"],
      "@/*": ["./src/*"]
    }
  },
  "include": ["src"],
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
    <meta name="description" content="nvbes Developers console for Identity OAuth integrations." />
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
import path from 'node:path';
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
    process.env.VITE_ACCOUNT_SERVICE_BASE_URL ||
    localEnv.VITE_ACCOUNT_SERVICE_BASE_URL ||
    rootEnv.VITE_ACCOUNT_SERVICE_BASE_URL ||
    'http://localhost:4000';

  return {
    plugins: [react(), tailwindcss(), devtoolsJson()],
    resolve: {
      alias: [
        {
          find: '@',
          replacement: path.resolve(__dirname, './src')
        },
        {
          find: '@nvbes/http-client',
          replacement: path.resolve(__dirname, '../../libs/ts/http-client/src/index.ts')
        },
        {
          find: '@nvbes/identity-client',
          replacement: path.resolve(__dirname, '../../libs/ts/identity-client/src/index.ts')
        },
        {
          find: '@nvbes/identity-sdk-web',
          replacement: path.resolve(__dirname, '../../libs/ts/identity-sdk-web/src/index.ts')
        },
        {
          find: '@nvbes/web-runtime',
          replacement: path.resolve(__dirname, '../../libs/ts/web-runtime/src/index.ts')
        },
        {
          find: '@nvbes/web-ui',
          replacement: path.resolve(__dirname, '../../libs/ts/web-ui/src/index.ts')
        }
      ]
    },
    server: {
      port: 5175,
      proxy: {
        '/api': accountServiceProxyTarget,
        '/oauth': accountServiceProxyTarget,
        '/.well-known': accountServiceProxyTarget
      }
    }
  };
});
```

- [ ] **Step 3: Add the initial React shell**

Create `apps/console-web/src/developer.router.tsx`:

```tsx
import { createRootRoute, createRoute, createRouter, Link, Outlet } from '@tanstack/react-router';

function RootShell() {
  return <Outlet />;
}

function DeveloperHomePage() {
  return (
    <main className="min-h-screen bg-background text-foreground">
      <section className="mx-auto flex min-h-screen w-full max-w-6xl flex-col gap-8 px-6 py-8">
        <nav className="flex items-center justify-between border-b border-border pb-4">
          <Link to="/" className="text-lg font-semibold">
            nvbes Developers
          </Link>
          <Link to="/console" className="rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground">
            Open console
          </Link>
        </nav>
        <div className="grid flex-1 items-center gap-6 md:grid-cols-[1fr_380px]">
          <div className="space-y-5">
            <p className="text-sm font-medium uppercase tracking-[0.14em] text-muted-foreground">
              Identity integrations
            </p>
            <h1 className="max-w-3xl text-5xl font-semibold tracking-normal">
              Govern OAuth apps, tokens, webhooks, and scopes.
            </h1>
            <p className="max-w-2xl text-base leading-7 text-muted-foreground">
              Manage nvbes Identity integrations from a dedicated developer console.
            </p>
          </div>
          <div className="grid gap-3 rounded-lg border border-border bg-card p-4">
            {['OAuth apps', 'Scope registry', 'Secret rotation', 'Token debugger'].map((label) => (
              <div key={label} className="rounded-md border border-border bg-background px-4 py-3 text-sm font-medium">
                {label}
              </div>
            ))}
          </div>
        </div>
      </section>
    </main>
  );
}

function ConsolePreviewPage() {
  return (
    <main className="min-h-screen bg-background px-6 py-8 text-foreground">
      <div className="mx-auto max-w-6xl">
        <h1 className="text-2xl font-semibold">Developer Console</h1>
        <p className="mt-2 text-sm text-muted-foreground">
          Console routes connect to `/api/v1/developer` in later tasks.
        </p>
      </div>
    </main>
  );
}

const rootRoute = createRootRoute({ component: RootShell });
const homeRoute = createRoute({ getParentRoute: () => rootRoute, path: '/', component: DeveloperHomePage });
const consoleRoute = createRoute({ getParentRoute: () => rootRoute, path: '/console', component: ConsolePreviewPage });
const routeTree = rootRoute.addChildren([homeRoute, consoleRoute]);

export const router = createRouter({ routeTree });

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router;
  }
}
```

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

Create `apps/console-web/src/styles.css` with the same import stack and theme pattern as `account-web`, using a restrained operational palette:

```css
@import 'tailwindcss';
@import 'tw-animate-css';
@import 'shadcn/tailwind.css';
@import '@fontsource-variable/geist';

@source '../../../libs/ts/web-runtime/src';
@source '../../../libs/ts/web-ui/src';
@source './';

@theme inline {
  --font-sans: 'Geist Variable', sans-serif;
  --color-background: var(--background);
  --color-foreground: var(--foreground);
  --color-card: var(--card);
  --color-card-foreground: var(--card-foreground);
  --color-primary: var(--primary);
  --color-primary-foreground: var(--primary-foreground);
  --color-muted: var(--muted);
  --color-muted-foreground: var(--muted-foreground);
  --color-border: var(--border);
  --color-input: var(--input);
  --color-ring: var(--ring);
  --radius-sm: calc(var(--radius) * 0.6);
  --radius-md: calc(var(--radius) * 0.8);
  --radius-lg: var(--radius);
}

:root {
  --background: oklch(0.982 0.004 210);
  --foreground: oklch(0.17 0.012 230);
  --card: oklch(1 0 0);
  --card-foreground: oklch(0.17 0.012 230);
  --primary: oklch(0.45 0.095 178);
  --primary-foreground: oklch(0.98 0.01 178);
  --muted: oklch(0.94 0.006 214);
  --muted-foreground: oklch(0.46 0.026 226);
  --border: oklch(0.88 0.01 218);
  --input: oklch(0.88 0.01 218);
  --ring: oklch(0.62 0.06 184);
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

- [ ] **Step 4: Wire scripts**

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

Modify root `package.json`:

- add `"dev:console-web": "bash scripts/dev-console-web.sh"`;
- add `console-web` to `build`, `build:web`, `build:analyze`, `check:size`, `check:web`, `format`, `format:check`, and `lint:web` project lists where web apps are enumerated.

- [ ] **Step 5: Verify the scaffold**

Run:

```bash
rtk pnpm exec nx show project console-web --json
rtk pnpm exec nx run console-web:format:check --skip-nx-cache
rtk pnpm exec nx run console-web:typecheck
rtk pnpm exec nx run console-web:lint
rtk pnpm exec nx run console-web:build --skip-nx-cache
```

Expected: Nx discovers `console-web`; format check, typecheck, lint, and build pass.

- [ ] **Step 6: Commit**

```bash
rtk git add apps/console-web package.json scripts/dev-console-web.sh
rtk git commit -m "feat(developer): scaffold console web app"
```

### Task 2: Add Developer RBAC And Persistence Foundations

**Files:**

- Create: `apps/account-service/migrations/0008_developer_console.sql`
- Create: `apps/account-service/src/identity.domains.developer.mod.rs`
- Create: `apps/account-service/src/identity.domains.developer.types.rs`
- Create: `apps/account-service/src/identity.domains.developer.rbac.rs`
- Create: `apps/account-service/src/identity.domains.developer.rbac.db.rs`
- Create: `apps/account-service/src/identity.domains.developer.routes.rs`
- Create: `apps/account-service/src/identity.domains.developer.service.rs`
- Create/replace: `apps/account-service/src/identity.domains.developer.tests.rs`
- Modify: `apps/account-service/src/identity.domains.mod.rs`

- [ ] **Step 1: Write failing RBAC tests**

Create or replace `apps/account-service/src/identity.domains.developer.tests.rs`:

```rust
use super::rbac::{DeveloperPermission, DeveloperRole, permissions_for_role};

#[test]
fn developer_admin_has_all_developer_permissions() {
    let permissions = permissions_for_role(DeveloperRole::DeveloperAdmin);

    assert!(permissions.contains(&DeveloperPermission::AppsCreate));
    assert!(permissions.contains(&DeveloperPermission::MarketplaceReview));
    assert!(permissions.contains(&DeveloperPermission::SecretsRotate));
    assert!(permissions.contains(&DeveloperPermission::WebhooksReplay));
    assert!(permissions.contains(&DeveloperPermission::LogsRead));
    assert!(permissions.contains(&DeveloperPermission::RbacManage));
}

#[test]
fn integration_tester_can_use_tools_without_mutating_apps() {
    let permissions = permissions_for_role(DeveloperRole::IntegrationTester);

    assert!(permissions.contains(&DeveloperPermission::AppsRead));
    assert!(permissions.contains(&DeveloperPermission::TokensInspect));
    assert!(permissions.contains(&DeveloperPermission::HealthChecksRun));
    assert!(permissions.contains(&DeveloperPermission::SandboxUse));
    assert!(!permissions.contains(&DeveloperPermission::AppsCreate));
    assert!(!permissions.contains(&DeveloperPermission::SecretsRotate));
    assert!(!permissions.contains(&DeveloperPermission::WebhooksManage));
}

#[test]
fn docs_viewer_has_only_docs_access() {
    let permissions = permissions_for_role(DeveloperRole::DocsViewer);

    assert_eq!(permissions, vec![DeveloperPermission::DocsRead]);
}
```

- [ ] **Step 2: Verify the test fails**

Run:

```bash
rtk cargo test -p nvbes-account-service developer_ --lib
```

Expected: failure because `identity.domains.developer.rbac` is not declared.

- [ ] **Step 3: Add the migration**

Create `apps/account-service/migrations/0008_developer_console.sql`:

```sql
CREATE TYPE developer_role AS ENUM (
  'developer_admin',
  'app_manager',
  'webhook_manager',
  'log_viewer',
  'integration_tester',
  'docs_viewer'
);

CREATE TYPE developer_marketplace_status AS ENUM ('pending', 'approved', 'rejected', 'suspended');
CREATE TYPE developer_scope_risk AS ENUM ('low', 'medium', 'high', 'restricted');
CREATE TYPE developer_scope_lifecycle AS ENUM ('proposed', 'active', 'deprecated', 'retired');
CREATE TYPE developer_webhook_status AS ENUM ('active', 'paused', 'revoked');
CREATE TYPE developer_delivery_status AS ENUM ('pending', 'delivered', 'failed', 'replayed');
CREATE TYPE developer_health_status AS ENUM ('passing', 'warning', 'failing', 'unknown');

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

CREATE INDEX idx_developer_role_assignments_active
  ON developer_role_assignments (tenant_id, principal_id)
  WHERE revoked_at IS NULL;

CREATE TABLE developer_marketplace_apps (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  client_id text NOT NULL REFERENCES oauth_clients(client_id) ON DELETE CASCADE,
  status developer_marketplace_status NOT NULL DEFAULT 'pending',
  submitted_by uuid REFERENCES principals(id) ON DELETE SET NULL,
  reviewed_by uuid REFERENCES principals(id) ON DELETE SET NULL,
  review_reason text,
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now(),
  UNIQUE (tenant_id, client_id)
);

CREATE TABLE developer_consent_screens (
  client_id text PRIMARY KEY REFERENCES oauth_clients(client_id) ON DELETE CASCADE,
  product_name text NOT NULL,
  logo_url text,
  support_url text,
  privacy_url text,
  terms_url text,
  description text NOT NULL DEFAULT '',
  updated_by uuid REFERENCES principals(id) ON DELETE SET NULL,
  updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE developer_scope_registry (
  scope_key text PRIMARY KEY,
  display_name text NOT NULL,
  description text NOT NULL,
  risk developer_scope_risk NOT NULL,
  owner_team text NOT NULL,
  lifecycle developer_scope_lifecycle NOT NULL DEFAULT 'proposed',
  allowed_audiences text[] NOT NULL DEFAULT '{}',
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE developer_secret_rotations (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  client_id text NOT NULL REFERENCES oauth_clients(client_id) ON DELETE CASCADE,
  active_version_id uuid NOT NULL DEFAULT gen_random_uuid(),
  previous_version_expires_at timestamptz NOT NULL,
  overlap_ends_at timestamptz NOT NULL,
  rotated_by uuid REFERENCES principals(id) ON DELETE SET NULL,
  created_at timestamptz NOT NULL DEFAULT now(),
  revoked_at timestamptz,
  CONSTRAINT developer_secret_rotation_overlap_check CHECK (overlap_ends_at = previous_version_expires_at)
);

CREATE TABLE developer_webhook_endpoints (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  name text NOT NULL,
  url text NOT NULL,
  signing_secret_ciphertext text NOT NULL,
  signing_secret_last4 text NOT NULL,
  status developer_webhook_status NOT NULL DEFAULT 'active',
  created_by uuid REFERENCES principals(id) ON DELETE SET NULL,
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now(),
  revoked_at timestamptz
);

CREATE INDEX idx_developer_webhook_endpoints_active
  ON developer_webhook_endpoints (tenant_id)
  WHERE revoked_at IS NULL;

CREATE TABLE developer_webhook_deliveries (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  endpoint_id uuid NOT NULL REFERENCES developer_webhook_endpoints(id) ON DELETE CASCADE,
  tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  event_id uuid NOT NULL,
  event_type text NOT NULL,
  status developer_delivery_status NOT NULL,
  attempt_count integer NOT NULL DEFAULT 0,
  response_status integer,
  error_message text,
  created_at timestamptz NOT NULL DEFAULT now(),
  delivered_at timestamptz,
  replayed_from_delivery_id uuid REFERENCES developer_webhook_deliveries(id) ON DELETE SET NULL
);

CREATE TABLE developer_sandbox_tenants (
  tenant_id uuid PRIMARY KEY REFERENCES tenants(id) ON DELETE CASCADE,
  sandbox_tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  status text NOT NULL DEFAULT 'ready',
  data_profile text NOT NULL DEFAULT 'minimal',
  reset_requested_at timestamptz,
  updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE developer_health_checks (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  target_type text NOT NULL,
  target_id text NOT NULL,
  check_kind text NOT NULL,
  status developer_health_status NOT NULL,
  summary text NOT NULL,
  checked_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX idx_developer_health_checks_latest
  ON developer_health_checks (tenant_id, target_type, target_id, check_kind, checked_at DESC);

CREATE TABLE developer_token_debug_sessions (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  actor_principal_id uuid NOT NULL REFERENCES principals(id) ON DELETE CASCADE,
  token_hash_prefix text NOT NULL,
  active boolean NOT NULL,
  access_decision text NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now()
);
```

- [ ] **Step 4: Add module declarations and RBAC**

Create `apps/account-service/src/identity.domains.developer.mod.rs`:

```rust
#[path = "identity.domains.developer.rbac.rs"]
pub mod rbac;
#[path = "identity.domains.developer.rbac.db.rs"]
pub mod rbac_db;
#[path = "identity.domains.developer.routes.rs"]
pub mod routes;
#[path = "identity.domains.developer.service.rs"]
pub mod service;
#[path = "identity.domains.developer.types.rs"]
pub mod types;

#[cfg(test)]
#[path = "identity.domains.developer.tests.rs"]
mod tests;
```

Create `apps/account-service/src/identity.domains.developer.rbac.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeveloperRole {
    DeveloperAdmin,
    AppManager,
    WebhookManager,
    LogViewer,
    IntegrationTester,
    DocsViewer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeveloperPermission {
    DocsRead,
    AppsRead,
    AppsCreate,
    AppsUpdate,
    AppsRevoke,
    MarketplaceRead,
    MarketplaceSubmit,
    MarketplaceReview,
    ScopesRead,
    ScopesManage,
    SecretsRotate,
    ServiceAccountsRead,
    ServiceAccountsManage,
    WebhooksRead,
    WebhooksManage,
    WebhooksReplay,
    LogsRead,
    TokensInspect,
    HealthChecksRead,
    HealthChecksRun,
    SandboxUse,
    RbacManage,
}

pub fn permissions_for_role(role: DeveloperRole) -> Vec<DeveloperPermission> {
    use DeveloperPermission::*;

    match role {
        DeveloperRole::DeveloperAdmin => vec![
            DocsRead, AppsRead, AppsCreate, AppsUpdate, AppsRevoke, MarketplaceRead,
            MarketplaceSubmit, MarketplaceReview, ScopesRead, ScopesManage, SecretsRotate,
            ServiceAccountsRead, ServiceAccountsManage, WebhooksRead, WebhooksManage,
            WebhooksReplay, LogsRead, TokensInspect, HealthChecksRead, HealthChecksRun,
            SandboxUse, RbacManage,
        ],
        DeveloperRole::AppManager => vec![
            DocsRead, AppsRead, AppsCreate, AppsUpdate, AppsRevoke, MarketplaceRead,
            MarketplaceSubmit, ScopesRead, SecretsRotate, ServiceAccountsRead,
            ServiceAccountsManage, HealthChecksRead,
        ],
        DeveloperRole::WebhookManager => vec![
            DocsRead, WebhooksRead, WebhooksManage, WebhooksReplay, LogsRead, HealthChecksRead,
            HealthChecksRun,
        ],
        DeveloperRole::LogViewer => vec![DocsRead, AppsRead, WebhooksRead, LogsRead, HealthChecksRead],
        DeveloperRole::IntegrationTester => vec![
            DocsRead, AppsRead, MarketplaceRead, ScopesRead, ServiceAccountsRead, WebhooksRead,
            LogsRead, TokensInspect, HealthChecksRead, HealthChecksRun, SandboxUse,
        ],
        DeveloperRole::DocsViewer => vec![DocsRead],
    }
}
```

Create `apps/account-service/src/identity.domains.developer.routes.rs`:

```rust
use axum::Router;

use crate::app::AppState;

pub fn router(_state: &AppState) -> Router<AppState> {
    Router::new()
}
```

Create `apps/account-service/src/identity.domains.developer.rbac.db.rs`:

```rust
// Database-backed developer role lookups are introduced with context routes.
```

Create `apps/account-service/src/identity.domains.developer.service.rs`:

```rust
// Developer facade orchestration helpers are introduced with the first API routes.
```

Modify `apps/account-service/src/identity.domains.mod.rs`:

```rust
#[path = "identity.domains.developer.mod.rs"]
pub mod developer;
```

Add `.merge(developer::routes::router(state))` to `router`.

- [ ] **Step 5: Verify RBAC tests pass**

Run:

```bash
rtk cargo test -p nvbes-account-service developer_ --lib
rtk cargo check --workspace
```

Expected: developer RBAC tests pass; workspace compiles.

- [ ] **Step 6: Commit**

```bash
rtk git add apps/account-service/migrations/0008_developer_console.sql apps/account-service/src/identity.domains.developer.* apps/account-service/src/identity.domains.mod.rs
rtk git commit -m "feat(identity): add developer console foundations"
```

### Task 3: Add Developer Context, API Schemas, And Console Shell

**Files:**

- Create: `apps/account-service/src/identity.domains.developer.routes.context.rs`
- Modify: `apps/account-service/src/identity.domains.developer.routes.rs`
- Modify: `apps/account-service/src/identity.domains.developer.types.rs`
- Modify: `apps/console-web/src/developer.schemas.ts`
- Modify: `apps/console-web/src/developer.api.ts`
- Modify: `apps/console-web/src/developer.permissions.ts`
- Create: `apps/console-web/src/layouts/DeveloperShell.tsx`
- Modify: `apps/console-web/src/developer.router.tsx`
- Create: `apps/console-web/src/pages/OverviewPage.tsx`
- Test: `apps/console-web/src/__tests__/developer.permissions.test.ts`

- [ ] **Step 1: Write failing frontend permission tests**

Create `apps/console-web/src/__tests__/developer.permissions.test.ts`:

```ts
import { describe, expect, test } from 'vitest';
import { canUseDeveloperRoute, summarizeDeveloperPermissions } from '../developer.permissions';

describe('developer permissions', () => {
  test('allows app managers to open OAuth apps and secrets routes', () => {
    const permissions = ['apps:read', 'apps:create', 'secrets:rotate'];

    expect(canUseDeveloperRoute(permissions, '/console/oauth-apps')).toBe(true);
    expect(canUseDeveloperRoute(permissions, '/console/secrets')).toBe(true);
    expect(canUseDeveloperRoute(permissions, '/console/developer-roles')).toBe(false);
  });

  test('summarizes missing developer role', () => {
    expect(summarizeDeveloperPermissions([])).toBe('No developer role assigned');
  });
});
```

- [ ] **Step 2: Verify frontend test fails**

Run:

```bash
rtk pnpm --dir apps/console-web exec vitest run src/__tests__/developer.permissions.test.ts
```

Expected: failure because `developer.permissions.ts` is missing.

- [ ] **Step 3: Add backend context DTOs and route**

In `apps/account-service/src/identity.domains.developer.types.rs`, define context DTOs:

```rust
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, ToSchema)]
pub struct DeveloperContextResponse {
    pub tenant_id: Uuid,
    pub principal_id: Uuid,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
    pub workspaces: Vec<DeveloperWorkspaceSummary>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DeveloperWorkspaceSummary {
    pub id: Uuid,
    pub name: String,
    pub role: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DeveloperOverviewResponse {
    pub oauth_clients: i64,
    pub marketplace_pending: i64,
    pub service_accounts: i64,
    pub failing_health_checks: i64,
    pub failed_webhook_deliveries: i64,
}
```

Create `apps/account-service/src/identity.domains.developer.routes.context.rs`:

```rust
use axum::{
    Extension, Json,
    extract::State,
};

use crate::{
    app::AppState,
    domains::developer::types::{DeveloperContextResponse, DeveloperOverviewResponse},
    http::{error::AppError, middleware::jwt::AuthContext},
};

pub async fn get_context(
    State(_state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<DeveloperContextResponse>, AppError> {
    let tenant_id = auth.tenant_id.ok_or_else(|| {
        AppError::forbidden("tenant_context_required", "Tenant context is required.")
    })?;

    Ok(Json(DeveloperContextResponse {
        tenant_id,
        principal_id: auth.user_id,
        roles: Vec::new(),
        permissions: Vec::new(),
        workspaces: Vec::new(),
    }))
}

pub async fn get_overview(
    State(_state): State<AppState>,
    Extension(_auth): Extension<AuthContext>,
) -> Result<Json<DeveloperOverviewResponse>, AppError> {
    Ok(Json(DeveloperOverviewResponse {
        oauth_clients: 0,
        marketplace_pending: 0,
        service_accounts: 0,
        failing_health_checks: 0,
        failed_webhook_deliveries: 0,
    }))
}
```

Modify `identity.domains.developer.routes.rs`:

```rust
use axum::{Router, routing::get};

use crate::app::AppState;

#[path = "identity.domains.developer.routes.context.rs"]
pub mod context;

pub fn router(state: &AppState) -> Router<AppState> {
    let auth_middleware = crate::http::middleware::jwt::jwt_auth_middleware;

    Router::new()
        .route("/api/v1/developer/context", get(context::get_context))
        .route("/api/v1/developer/overview", get(context::get_overview))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
}
```

- [ ] **Step 4: Add frontend schemas, API, and shell**

Create `apps/console-web/src/developer.schemas.ts`:

```ts
import { z } from 'zod';

export const DeveloperContextSchema = z.object({
  tenant_id: z.string(),
  principal_id: z.string(),
  roles: z.array(z.string()),
  permissions: z.array(z.string()),
  workspaces: z.array(z.object({ id: z.string(), name: z.string(), role: z.string() }))
});

export const DeveloperOverviewSchema = z.object({
  oauth_clients: z.number(),
  marketplace_pending: z.number(),
  service_accounts: z.number(),
  failing_health_checks: z.number(),
  failed_webhook_deliveries: z.number()
});

export type DeveloperContext = z.infer<typeof DeveloperContextSchema>;
export type DeveloperOverview = z.infer<typeof DeveloperOverviewSchema>;
```

Create `apps/console-web/src/developer.api.ts`:

```ts
import { identityHttpClient } from './identity.http';
import { DeveloperContextSchema, DeveloperOverviewSchema } from './developer.schemas';

export function getDeveloperContext() {
  return identityHttpClient.get('/api/v1/developer/context', DeveloperContextSchema);
}

export function getDeveloperOverview() {
  return identityHttpClient.get('/api/v1/developer/overview', DeveloperOverviewSchema);
}
```

Create `apps/console-web/src/identity.http.ts` using the same pattern as `account-web/src/identity.http.ts`: `VITE_ACCOUNT_SERVICE_BASE_URL`, fallback `http://localhost:4000`, and `credentials: 'include'`. Developer API functions include the `/api/v1/developer` route prefix explicitly.

Create `apps/console-web/src/developer.permissions.ts`:

```ts
const routeRequirements: Record<string, string> = {
  '/console/oauth-apps': 'apps:read',
  '/console/marketplace': 'marketplace:read',
  '/console/scopes': 'scopes:read',
  '/console/service-accounts': 'service_accounts:read',
  '/console/secrets': 'secrets:rotate',
  '/console/webhooks': 'webhooks:read',
  '/console/logs': 'logs:read',
  '/console/token-debugger': 'tokens:inspect',
  '/console/sandbox': 'sandbox:use',
  '/console/health-checks': 'health_checks:read',
  '/console/developer-roles': 'rbac:manage'
};

export function canUseDeveloperRoute(permissions: readonly string[], pathname: string): boolean {
  const required = routeRequirements[pathname];
  return !required || permissions.includes(required);
}

export function summarizeDeveloperPermissions(permissions: readonly string[]): string {
  if (permissions.length === 0) return 'No developer role assigned';
  return `${permissions.length} developer permissions`;
}
```

Create `DeveloperShell.tsx` and `OverviewPage.tsx` with a sidebar and overview query. Keep pages under 300 lines.

- [ ] **Step 5: Verify context shell**

Run:

```bash
rtk pnpm --dir apps/console-web exec vitest run src/__tests__/developer.permissions.test.ts
rtk pnpm exec nx run console-web:typecheck
rtk cargo check --workspace
```

Expected: frontend permission tests pass; developer app typechecks; Rust compiles.

- [ ] **Step 6: Commit**

```bash
rtk git add apps/console-web/src apps/account-service/src/identity.domains.developer.*
rtk git commit -m "feat(developer): add console context shell"
```

### Task 4: Add OAuth Clients, Marketplace, Consent Screens, And Scope Registry

**Files:**

- Modify: `apps/account-service/src/identity.domains.developer.types.rs`
- Create: `apps/account-service/src/identity.domains.developer.routes.oauth.rs`
- Modify: `apps/account-service/src/identity.domains.developer.routes.rs`
- Modify: `apps/console-web/src/developer.schemas.ts`
- Modify: `apps/console-web/src/developer.api.ts`
- Create: `apps/console-web/src/pages/OAuthAppsPage.tsx`
- Create: `apps/console-web/src/pages/MarketplacePage.tsx`
- Create: `apps/console-web/src/pages/ScopesPage.tsx`
- Test: `apps/console-web/src/__tests__/developer.schemas.test.ts`

- [ ] **Step 1: Write failing schema tests**

Create `apps/console-web/src/__tests__/developer.schemas.test.ts`:

```ts
import { describe, expect, test } from 'vitest';
import {
  DeveloperOAuthClientsSchema,
  DeveloperScopeRegistrySchema
} from '../developer.schemas';

describe('developer schemas', () => {
  test('parses OAuth client summaries with marketplace and consent status', () => {
    const result = DeveloperOAuthClientsSchema.parse({
      oauth_clients: [
        {
          client_id: 'client_123',
          name: 'Drive Sync',
          status: 'active',
          marketplace_status: 'approved',
          consent_screen_configured: true,
          redirect_uri_count: 2,
          allowed_scopes: ['openid', 'drive.files.read'],
          health_status: 'passing'
        }
      ]
    });

    expect(result.oauth_clients[0].marketplace_status).toBe('approved');
  });

  test('parses scope registry risk and lifecycle metadata', () => {
    const result = DeveloperScopeRegistrySchema.parse({
      scopes: [
        {
          scope_key: 'drive.files.read',
          display_name: 'Read files',
          description: 'Read Drive file metadata and content.',
          risk: 'medium',
          owner_team: 'drive',
          lifecycle: 'active',
          allowed_audiences: ['cloud-service']
        }
      ]
    });

    expect(result.scopes[0].risk).toBe('medium');
  });
});
```

- [ ] **Step 2: Verify schema tests fail**

Run:

```bash
rtk pnpm --dir apps/console-web exec vitest run src/__tests__/developer.schemas.test.ts
```

Expected: failure because OAuth and scope schemas are missing.

- [ ] **Step 3: Add backend DTOs and routes**

Add DTOs for:

```rust
pub struct DeveloperOAuthClientsResponse { pub oauth_clients: Vec<DeveloperOAuthClientSummary> }
pub struct DeveloperOAuthClientSummary {
    pub client_id: String,
    pub name: String,
    pub status: String,
    pub marketplace_status: Option<String>,
    pub consent_screen_configured: bool,
    pub redirect_uri_count: i64,
    pub allowed_scopes: Vec<String>,
    pub health_status: String,
}
pub struct DeveloperScopeRegistryResponse { pub scopes: Vec<DeveloperScopeRegistryEntry> }
pub struct DeveloperScopeRegistryEntry {
    pub scope_key: String,
    pub display_name: String,
    pub description: String,
    pub risk: String,
    pub owner_team: String,
    pub lifecycle: String,
    pub allowed_audiences: Vec<String>,
}
```

Create routes:

- `GET /api/v1/developer/oauth-clients`
- `GET /api/v1/developer/marketplace/apps`
- `GET /api/v1/developer/scopes`
- `GET /api/v1/developer/oauth-clients/{clientId}/consent-screen`
- `PUT /api/v1/developer/oauth-clients/{clientId}/consent-screen`

Use tenant-scoped SQL joins across `oauth_clients`, `developer_marketplace_apps`, `developer_consent_screens`, and `developer_health_checks`.

- [ ] **Step 4: Add frontend schemas and pages**

Add Zod schemas matching the DTOs:

```ts
const HealthStatusSchema = z.enum(['passing', 'warning', 'failing', 'unknown']);
const MarketplaceStatusSchema = z.enum(['pending', 'approved', 'rejected', 'suspended']).nullable();

export const DeveloperOAuthClientSchema = z.object({
  client_id: z.string(),
  name: z.string(),
  status: z.string(),
  marketplace_status: MarketplaceStatusSchema,
  consent_screen_configured: z.boolean(),
  redirect_uri_count: z.number(),
  allowed_scopes: z.array(z.string()),
  health_status: HealthStatusSchema
});

export const DeveloperOAuthClientsSchema = z.object({
  oauth_clients: z.array(DeveloperOAuthClientSchema)
});

export const DeveloperScopeRegistrySchema = z.object({
  scopes: z.array(z.object({
    scope_key: z.string(),
    display_name: z.string(),
    description: z.string(),
    risk: z.enum(['low', 'medium', 'high', 'restricted']),
    owner_team: z.string(),
    lifecycle: z.enum(['proposed', 'active', 'deprecated', 'retired']),
    allowed_audiences: z.array(z.string())
  }))
});
```

Add API functions:

```ts
export function listDeveloperOAuthClients() {
  return identityHttpClient.get('/api/v1/developer/oauth-clients', DeveloperOAuthClientsSchema);
}

export function listDeveloperScopes() {
  return identityHttpClient.get('/api/v1/developer/scopes', DeveloperScopeRegistrySchema);
}
```

Build pages with compact tables, filters, and empty states. Use no `any`.

- [ ] **Step 5: Verify**

Run:

```bash
rtk pnpm --dir apps/console-web exec vitest run src/__tests__/developer.schemas.test.ts
rtk pnpm exec nx run console-web:typecheck
rtk cargo check --workspace
```

Expected: schema tests pass; web and Rust checks pass.

- [ ] **Step 6: Commit**

```bash
rtk git add apps/console-web/src apps/account-service/src/identity.domains.developer.*
rtk git commit -m "feat(developer): add oauth governance views"
```

### Task 5: Add Service Accounts And Secret Rotation

**Files:**

- Modify: `apps/account-service/src/identity.domains.developer.types.rs`
- Create: `apps/account-service/src/identity.domains.developer.routes.service_accounts.rs`
- Modify: `apps/account-service/src/identity.domains.developer.routes.rs`
- Modify: `apps/console-web/src/developer.schemas.ts`
- Modify: `apps/console-web/src/developer.api.ts`
- Create: `apps/console-web/src/pages/ServiceAccountsPage.tsx`
- Create: `apps/console-web/src/pages/SecretsPage.tsx`
- Test: `apps/console-web/src/__tests__/developer.secret-rotation.test.ts`

- [ ] **Step 1: Write failing secret rotation helper test**

Create `apps/console-web/src/__tests__/developer.secret-rotation.test.ts`:

```ts
import { describe, expect, test } from 'vitest';
import { buildSecretRotationPayload } from '../pages/SecretsPage.helpers';

describe('secret rotation helper', () => {
  test('requires an overlap window of at least one hour', () => {
    expect(() => buildSecretRotationPayload({ overlapHours: 0 })).toThrow('Overlap must be at least 1 hour');
  });

  test('builds payload with explicit overlap hours', () => {
    expect(buildSecretRotationPayload({ overlapHours: 24 })).toEqual({ overlap_hours: 24 });
  });
});
```

- [ ] **Step 2: Verify helper test fails**

Run:

```bash
rtk pnpm --dir apps/console-web exec vitest run src/__tests__/developer.secret-rotation.test.ts
```

Expected: failure because `SecretsPage.helpers` is missing.

- [ ] **Step 3: Add backend routes**

Add routes:

- `GET /api/v1/developer/service-accounts`
- `GET /api/v1/developer/service-accounts/{principalId}`
- `POST /api/v1/developer/service-accounts`
- `POST /api/v1/developer/service-accounts/{principalId}/oauth-clients`
- `POST /api/v1/developer/oauth-clients/{clientId}/secrets/rotation`
- `POST /api/v1/developer/oauth-clients/{clientId}/secrets/{versionId}/revoke`

The service-account routes delegate to `domains::service_accounts::service` after developer permission checks. The rotation route delegates to the existing rotate-secret primitive when possible and records overlap metadata in `developer_secret_rotations`.

- [ ] **Step 4: Add frontend schemas, helpers, and pages**

Create `apps/console-web/src/pages/SecretsPage.helpers.ts`:

```ts
export type SecretRotationForm = {
  overlapHours: number;
};

export function buildSecretRotationPayload(form: SecretRotationForm) {
  if (!Number.isInteger(form.overlapHours) || form.overlapHours < 1) {
    throw new Error('Overlap must be at least 1 hour');
  }

  return { overlap_hours: form.overlapHours };
}
```

Add schemas for service accounts and rotation result, reuse names compatible with `account-web` service-account schemas where fields match.

- [ ] **Step 5: Verify**

Run:

```bash
rtk pnpm --dir apps/console-web exec vitest run src/__tests__/developer.secret-rotation.test.ts
rtk pnpm exec nx run console-web:typecheck
rtk cargo check --workspace
```

Expected: helper test passes; web and Rust checks pass.

- [ ] **Step 6: Commit**

```bash
rtk git add apps/console-web/src apps/account-service/src/identity.domains.developer.*
rtk git commit -m "feat(developer): add service account secret workflows"
```

### Task 6: Add Webhook Replay And Integration Logs

**Files:**

- Modify: `apps/account-service/src/identity.domains.developer.types.rs`
- Create: `apps/account-service/src/identity.domains.developer.routes.webhooks.rs`
- Modify: `apps/account-service/src/identity.domains.developer.routes.rs`
- Modify: `apps/console-web/src/developer.schemas.ts`
- Modify: `apps/console-web/src/developer.api.ts`
- Create: `apps/console-web/src/pages/WebhooksPage.tsx`
- Create: `apps/console-web/src/pages/LogsPage.tsx`
- Test: `apps/console-web/src/__tests__/developer.webhooks.test.ts`

- [ ] **Step 1: Write failing replay eligibility test**

Create `apps/console-web/src/__tests__/developer.webhooks.test.ts`:

```ts
import { describe, expect, test } from 'vitest';
import { canReplayWebhookDelivery } from '../pages/WebhooksPage.helpers';

describe('webhook replay eligibility', () => {
  test('allows failed deliveries', () => {
    expect(canReplayWebhookDelivery({ status: 'failed', attempt_count: 2 })).toBe(true);
  });

  test('blocks delivered deliveries', () => {
    expect(canReplayWebhookDelivery({ status: 'delivered', attempt_count: 1 })).toBe(false);
  });
});
```

- [ ] **Step 2: Verify replay test fails**

Run:

```bash
rtk pnpm --dir apps/console-web exec vitest run src/__tests__/developer.webhooks.test.ts
```

Expected: failure because helper is missing.

- [ ] **Step 3: Add backend webhooks and logs routes**

Add routes:

- `GET /api/v1/developer/webhooks`
- `POST /api/v1/developer/webhooks`
- `PATCH /api/v1/developer/webhooks/{endpointId}`
- `GET /api/v1/developer/webhooks/{endpointId}/deliveries`
- `POST /api/v1/developer/webhooks/deliveries/{deliveryId}/replay`
- `GET /api/v1/developer/logs`

Replay route rules:

- delivery must exist in the same tenant;
- status must be `failed` or `pending`;
- original endpoint must not be revoked;
- new delivery row sets `replayed_from_delivery_id`;
- audit event records actor, delivery id, endpoint id, and event id.

- [ ] **Step 4: Add frontend helpers and pages**

Create `apps/console-web/src/pages/WebhooksPage.helpers.ts`:

```ts
export type ReplayableDelivery = {
  status: 'pending' | 'delivered' | 'failed' | 'replayed';
  attempt_count: number;
};

export function canReplayWebhookDelivery(delivery: ReplayableDelivery): boolean {
  return delivery.status === 'failed' || delivery.status === 'pending';
}
```

Add Webhooks page with endpoint table, delivery table, and replay action. Add Logs page with filters for client, service account, event type, severity, and date.

- [ ] **Step 5: Verify**

Run:

```bash
rtk pnpm --dir apps/console-web exec vitest run src/__tests__/developer.webhooks.test.ts
rtk pnpm exec nx run console-web:typecheck
rtk cargo check --workspace
```

Expected: replay tests pass; checks pass.

- [ ] **Step 6: Commit**

```bash
rtk git add apps/console-web/src apps/account-service/src/identity.domains.developer.*
rtk git commit -m "feat(developer): add webhook replay logs"
```

### Task 7: Add Token Debugger, Sandbox, And Health Checks

**Files:**

- Modify: `apps/account-service/src/identity.domains.developer.types.rs`
- Create: `apps/account-service/src/identity.domains.developer.routes.tools.rs`
- Create: `apps/account-service/src/identity.domains.developer.token_debugger.rs`
- Create: `apps/account-service/src/identity.domains.developer.health.rs`
- Modify: `apps/account-service/src/identity.domains.developer.routes.rs`
- Modify: `apps/console-web/src/developer.schemas.ts`
- Modify: `apps/console-web/src/developer.api.ts`
- Create: `apps/console-web/src/pages/TokenDebuggerPage.tsx`
- Create: `apps/console-web/src/pages/SandboxPage.tsx`
- Create: `apps/console-web/src/pages/HealthChecksPage.tsx`
- Test: `apps/console-web/src/__tests__/developer.health.test.ts`

- [ ] **Step 1: Write failing health status test**

Create `apps/console-web/src/__tests__/developer.health.test.ts`:

```ts
import { describe, expect, test } from 'vitest';
import { summarizeHealthStatus } from '../pages/HealthChecksPage.helpers';

describe('health status summary', () => {
  test('prioritizes failing checks', () => {
    expect(summarizeHealthStatus(['passing', 'warning', 'failing'])).toBe('failing');
  });

  test('returns passing when all checks pass', () => {
    expect(summarizeHealthStatus(['passing', 'passing'])).toBe('passing');
  });
});
```

- [ ] **Step 2: Verify health test fails**

Run:

```bash
rtk pnpm --dir apps/console-web exec vitest run src/__tests__/developer.health.test.ts
```

Expected: failure because helper is missing.

- [ ] **Step 3: Add backend token debugger and health normalization**

Token debugger:

- accepts `{ token, expected_audience, required_scope }`;
- validates access tokens with existing JWT service;
- can call existing introspection logic when client context exists;
- returns active state, claims, scopes, audience, expiry, and access decision;
- stores only token hash prefix and decision in `developer_token_debug_sessions`.

Health checks:

- redirect URI check: validates scheme, host, and registered redirect URI shape;
- JWKS check: fetches `/.well-known/jwks.json` for configured issuer;
- OIDC discovery check: fetches `/.well-known/openid-configuration`;
- webhook check: validates endpoint URL shape and latest delivery status;
- SCIM check: reports `unknown` if no connector exists and `failing` for invalid configured connector.

- [ ] **Step 4: Add frontend helpers and pages**

Create `apps/console-web/src/pages/HealthChecksPage.helpers.ts`:

```ts
export type HealthStatus = 'passing' | 'warning' | 'failing' | 'unknown';

export function summarizeHealthStatus(statuses: readonly HealthStatus[]): HealthStatus {
  if (statuses.includes('failing')) return 'failing';
  if (statuses.includes('warning')) return 'warning';
  if (statuses.includes('unknown')) return 'unknown';
  return 'passing';
}
```

Add Token Debugger page with a textarea, expected audience input, required scope input, and result panels for claims, scopes, expiry, and access decision. Do not persist token values in browser storage.

Add Sandbox page with current sandbox binding, data profile, and reset request action.

Add Health Checks page with target filters and run button.

- [ ] **Step 5: Verify**

Run:

```bash
rtk pnpm --dir apps/console-web exec vitest run src/__tests__/developer.health.test.ts
rtk pnpm exec nx run console-web:typecheck
rtk cargo check --workspace
```

Expected: health tests pass; checks pass.

- [ ] **Step 6: Commit**

```bash
rtk git add apps/console-web/src apps/account-service/src/identity.domains.developer.*
rtk git commit -m "feat(developer): add token debugging health checks"
```

### Task 8: Add Developer Roles UI, OpenAPI, And Docs

**Files:**

- Modify: `apps/account-service/src/identity.http.openapi.rs`
- Modify: `libs/ts/identity-sdk-core/openapi.json`
- Modify: `libs/ts/identity-sdk-core/src/types.gen.ts`
- Create: `apps/console-web/src/pages/DeveloperRolesPage.tsx`
- Modify: `apps/console-web/src/developer.router.tsx`
- Create: `docs/api/identity-developer-console.md`
- Modify: `libs/ts/identity-sdk/README.md`

- [ ] **Step 1: Add developer role page**

Create `DeveloperRolesPage.tsx` showing current role assignments and permission groups from `/developer/context`. Role mutation can be read-only in V0 unless RBAC assignment routes were completed in Task 2 implementation.

- [ ] **Step 2: Register OpenAPI paths**

Modify `identity.http.openapi.rs` to include every new developer route handler and DTO type in the utoipa document.

Run:

```bash
rtk pnpm generate:openapi
```

Expected: `libs/ts/identity-sdk-core/openapi.json` and generated types update without generator errors.

- [ ] **Step 3: Add API docs**

Create `docs/api/identity-developer-console.md` with:

- route prefix;
- role-to-permission matrix;
- endpoint table;
- token debugger storage rule;
- webhook replay eligibility;
- secret rotation overlap behavior.

- [ ] **Step 4: Verify**

Run:

```bash
rtk pnpm exec nx run console-web:typecheck
rtk cargo check --workspace
rtk pnpm test:smoke:contract
```

Expected: typecheck and cargo check pass; OpenAPI contract smoke passes.

- [ ] **Step 5: Commit**

```bash
rtk git add apps/console-web/src apps/account-service/src/identity.http.openapi.rs libs/ts/identity-sdk-core docs/api/identity-developer-console.md libs/ts/identity-sdk/README.md
rtk git commit -m "docs(developer): document identity developer console"
```

### Task 9: Final Validation And Frontend Browser QA

**Files:**

- Modify only files needed for issues found during validation.

- [ ] **Step 1: Run focused checks**

Run:

```bash
rtk pnpm exec nx run console-web:format:check
rtk pnpm exec nx run console-web:lint
rtk pnpm exec nx run console-web:typecheck
rtk cargo test -p nvbes-account-service developer_ --lib
rtk cargo check --workspace
```

Expected: all commands pass.

- [ ] **Step 2: Run root affected checks**

Run:

```bash
rtk pnpm nx:affected
```

Expected: affected lint, typecheck, test, and build targets pass. If unrelated existing failures appear, record the failing project, target, and first error line.

- [ ] **Step 3: Start local app**

Run:

```bash
rtk pnpm dev:console-web
```

Open `http://localhost:5175` in the in-app browser. Verify:

- landing page renders;
- console route renders;
- navigation labels fit on desktop and mobile widths;
- no blank page;
- no overlapping text;
- empty/error/loading states are visible when API is unavailable.

- [ ] **Step 4: Final commit for validation fixes**

If Step 1 or browser QA required fixes:

```bash
rtk git add apps/console-web apps/account-service docs libs package.json scripts
rtk git commit -m "fix(developer): polish console validation issues"
```

Skip this commit if validation required no code changes.
