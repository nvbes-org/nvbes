# Architecture Memory

## Canonical Instructions

`AGENTS.md` is the canonical source of truth for all LLM agents and developers.

## V1 Ultra-Restricted Architecture (Small Building)

- **Monolithic API** (`apps/api`): Axum HTTP service bundling Identity/OAuth 2.0 (PKCE,
  authorization code flow, tokens), Consent management, and User Profile.
- **Monolithic Web** (`apps/web` / dev UI): React 19 + Vite + Tailwind + shadcn/ui.
  Provides Register, Login, Consent prompt, and Profile views. No marketing pages.
- **Data Persistence**: Standard PostgreSQL database (single connection pool, isolated schemas).
- **Two Major Principles**:
  1. *Frugality & Market Efficiency*: Tiny RAM footprint (< 20MB), free tier CDN (Cloudflare Pages),
     low-cost managed DB. Monthly spend strictly < 30 EUR TTC.
  2. *OSS Migration Readiness*: Zero vendor lock-in. Architecture directly swappable with
     self-hosted open-source components (PostgreSQL bare-metal/dedicated, Nginx/Caddy, SMTP).

## Platform Boundaries & Dormant Modules

- Billing (Stripe), Trust/Risk engine, and separate background workers are paused/dormant in V1.
- In-process background tasks use Tokio tasks rather than distributed queues.
- Enterprise governance, SAML/SCIM, and complex multi-tenancy remain future extensions.
