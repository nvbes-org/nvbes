# Project Memory

nvbes is a Rust/Axum and TypeScript/React monorepo managed with pnpm workspaces,
Nx, and a Cargo workspace.

## Active V1 Direction

- V1 delivers the reusable platform foundation, not a final user product.
- The shared domains are Identity, Account, Billing, Email, Trust/Risk, and
  minimal Platform Operations.
- The audience is general-public B2C with team collaboration. B2B, Enterprise,
  SLA, SSO/SCIM, and advanced compliance are future work.
- Cloud/Drive is a possible future product, not the assumed first product.
- Total recurring project spend is capped at EUR 30 including taxes, with a
  EUR 20 operating target.
- A solo operator handles administration, support, abuse, and moderation
  manually unless safe automation fits the global budget and answers a measured
  need.
- V1 accepts no known technical or structural debt in delivered scope;
  subsequent debt is owned, measured, reviewed, and resolved proactively.

## Stable Boundaries

- `apps/email-worker` and `apps/trust-risk-service` are the active runtimes.
- Historical product applications live under `archive/apps` and are not active
  implementation bases.
- `libs/rust/*` contains shared Rust primitives, adapters, and domain libraries.
- `libs/ts/*` contains SDKs, clients, UI, and web runtime packages.

## Durable Rules

- Rust files use flat dot-qualified filenames in `src/`.
- TypeScript must not use `any`.
- Delivered V1 code must not accept known temporary debt in touched areas.
- Agent instructions should flow from `docs/agent/*`.
