# NVBES API - Serverless Modular Monolith

Unified production HTTP API serving Identity, Account, Consent, and Billing capabilities as a zero-CLI, scale-to-zero serverless container.

## Architecture

- **Zero CLI**: The container does not use command-line arguments or subcommands. It starts immediately by binding to `$PORT` (default `8080`).
- **Serverless & Scale-to-Zero**:
  - Connection pool with `min_connections = 0` to release idle database connections when scaled down.
  - Stateless execution suitable for Google Cloud Run, Scaleway Serverless Containers, or Fly.io.
  - Graceful shutdown handling `SIGTERM` / `SIGINT` signals to flush in-flight HTTP requests.
- **Modular Monolith**:
  - Domain capabilities live in cohesive domain libraries and crates.
  - Sub-routers are mounted dynamically into the Axum root router.

## Endpoints

| Path | Domain | Description |
|---|---|---|
| `/healthz` | Health | Liveness probe (200 OK) |
| `/readyz` | Health | Readiness probe (database ping) |
| `/api/docs` | OpenAPI | Swagger UI documentation |
| `/api/docs/openapi.json` | OpenAPI | OpenAPI 3.0 JSON specification |
| `/api/v1/consents` | Consent | RGPD/ePrivacy agreements & cookie preferences with GPC |
| `/api/v1/profile` | Account | User profile management |
| `/api/v1/teams` | Account | Team spaces and membership |
| `/billing/plans` | Billing | Active subscription plans |
| `/webhooks/stripe` | Billing | Stripe webhook ingestion |
| `/.well-known/*` | Identity | OpenID Connect discovery and JWKS |
| `/oauth/*` | Identity | Token exchange, authorization, sessions, WebAuthn |

## Environment Variables

- `PORT`: HTTP port to listen on (default `8080`).
- `DATABASE_URL`: Postgres database connection string.
- `NVBES_ENVIRONMENT`: `production`, `development`, or `test`.
- `NVBES_API_DATABASE_MAX_CONNECTIONS`: Maximum database connections (default `10`).
- `NVBES_API_DATABASE_MIN_CONNECTIONS`: Minimum database connections (default `0` for scale-to-zero).
