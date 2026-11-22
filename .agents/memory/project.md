# Project Memory

nvbes is an entity building open-source tools and software for developers,
funded by its future cloud offering. The monorepo is managed with pnpm
workspaces, Nx, and a Cargo workspace (Rust/Axum + React/Vite).

## Active V1 Direction (Ultra-Restricted "Small Building")

- **Mission**: Open-source developer tools, funded by the future cloud.
- **V1 Target**: Identity & account service for developers using Nvbes tools.
  Manual / internal configuration of OAuth 2 clients (no self-serve developer portal).
- **Architecture**:
  - **Monolithic API** (`apps/api`): Account creation, login, session management
    all via OAuth 2 (PKCE RFC 7636), consent management (scopes), user profile.
  - **Monolithic Web** (`apps/web` / unified UI): Register, OAuth 2 login, consent
    screen, profile page. Strict dev-first UX, **no marketing front-end in V1**.
- **Two Major Principles**:
  1. **Maximum cost-efficiency**: Use the cheapest / most cost-effective technologies
     on the market (Rust/Axum < 20MB RAM, Cloudflare Pages free tier, managed low-cost PG).
     Monthly infrastructure spend hard-capped at 30 EUR TTC (target 20 EUR).
  2. **Migration path to self-hosted OSS**: Every technology must be swappable with
     self-hosted open-source software (native PostgreSQL, Nginx/Caddy, no proprietary lock-in).
- **Frozen / Out of V1 Scope**:
  - Billing (Stripe) is paused until the paid cloud launches.
  - No separate Trust/Risk engine or distributed background workers.
  - No marketing frontend, no complex B2B/Enterprise orgs or advanced compliance.
- **Operations & Quality**:
  - Solo operator with manual administration/support by default.
  - Zero technical debt delivered in active scope.

## Stable Boundaries

- `apps/api` is the active monolithic API runtime.
- `apps/identity-web` and `apps/account-web` are the web frontends (to be unified dev-first).
- `libs/rust/*` contains shared domains (identity, account, consent, core) and primitives.
- `libs/ts/*` contains SDKs, clients, and web runtime packages.
