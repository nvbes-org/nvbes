# Architecture Memory

## Canonical Instructions

`AGENTS.md` is the canonical source of truth for all LLM agents and developers.

## V1 Ultra-Restricted Architecture (Small Building)

- **Serverless API** (`apps/api`): Axum, Identity (OAuth 2.0 + PKCE, sessions,
  MFA, password recovery) and Account (profile, legal consents, RGPD). No
  background task, no migration. Account verifies tokens in-process through
  `ActivityVerifier` (identity SDK) implemented on `TokenService::introspect`.
- **Single worker** (`apps/worker`): `serve` for a Scaleway Container
  (recurring `identity-emails`, `privacy-retention`), `run <task>` for a
  Scaleway Job (`migrate`, `rotate-mfa-key`, or any recurring task once).
  Emails are rendered and sent in-process (`DirectEmailDelivery`: SMTP or
  Scaleway TEM); the old gRPC email service is archived.
- **Web**: `apps/identity-web` (hosted login) and `apps/account-web`; React 19,
  Vite, TanStack Router, shadcn/ui. Unification is a later step.
- **Data**: one PostgreSQL database, one schema per domain (`identity`,
  `account`), each with its own migration table.
- **Fail-closed configuration**: config loaders are pure `from_lookup`
  functions; outside development nothing is generated. Shared secrets (MFA,
  rate-limit key) must match across API instances and the worker.
- **CORS**: no global layer; Identity allows registered client origins,
  Account its configured browser origins.
- **Client IP**: only peers inside `NVBES_TRUSTED_PROXY_CIDRS` may supply
  `X-Forwarded-For`; the nearest untrusted hop wins.

## Archived (not compiled)

billing, billing-service, trust-risk, region, platform, consent (unauthenticated
subject header), audit (unused once consent left), the identity binary and the
account outbox (no consumer). They live in `archive/` and are marked
`archived` in the threshold files.
