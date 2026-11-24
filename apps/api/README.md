# NVBES API - Serverless Modular Monolith

Serverless HTTP API serving the Identity and Account domains as a scale-to-zero container. It runs no background task and never migrates: `nvbes-worker` owns both.

## Architecture

- **Fail-closed start**: every Identity secret, OAuth client registry, trusted proxy list and Account origin is validated before the listener binds; outside development nothing is generated.
- **Zero CLI**: The container starts immediately by binding to `$PORT` (default `8080`); `--export-openapi` prints the OpenAPI document.
- **Serverless & Scale-to-Zero**:
  - Connection pool with `min_connections = 0` to release idle database connections when scaled down.
  - Stateless execution suitable for Google Cloud Run, Scaleway Serverless Containers, or Fly.io.
  - Graceful shutdown handling `SIGTERM` / `SIGINT` signals to flush in-flight HTTP requests.
- **Modular Monolith**:
  - Domain capabilities live in cohesive domain libraries and crates.
  - Each domain owns its router and CORS policy; Account checks token activity in-process.

## Endpoints

| Path | Domain | Description |
|---|---|---|
| `/healthz` | Health | Liveness probe (200 OK) |
| `/readyz` | Health | Readiness probe (database ping) |
| `/api/docs` | OpenAPI | Swagger UI documentation |
| `/api/docs/openapi.json` | OpenAPI | OpenAPI 3.0 JSON specification |
| `/api/v1/consents` | Account | Authenticated, audited legal acceptances |
| `/api/v1/privacy/*`, `/api/v1/closure*` | Account | RGPD export and account closure |
| `/oauth/password-recovery/*` | Identity | Hosted password recovery |
| `/api/v1/profile` | Account | User profile management |
| `/.well-known/*` | Identity | OpenID Connect discovery and JWKS |
| `/oauth/*` | Identity | Token exchange, authorization, sessions, WebAuthn |

## Environment Variables

- `PORT`: HTTP port to listen on (default `8080`).
- `DATABASE_URL`: Postgres database connection string.
- `NVBES_ENVIRONMENT`: `production`, `development`, or `test`.
- `NVBES_API_DATABASE_MAX_CONNECTIONS`: Maximum database connections (default `10`).
- `NVBES_API_DATABASE_MIN_CONNECTIONS`: Minimum database connections (default `0` for scale-to-zero).
- `NVBES_TRUSTED_PROXY_CIDRS`: proxies allowed to set `X-Forwarded-For` (required outside development).
- `NVBES_ACCOUNT_BROWSER_ORIGINS_JSON`, `NVBES_ACCOUNT_PUBLIC_ORIGIN`: Account browser and DPoP origins (required outside development).
- `NVBES_IDENTITY_TOKEN_*`, `NVBES_IDENTITY_OAUTH_CLIENTS_JSON`, `NVBES_IDENTITY_BROWSER_ORIGIN`, `NVBES_IDENTITY_RATE_LIMIT_KEY`, `NVBES_IDENTITY_MFA_ENCRYPTION_KEY`: Identity signing keys, registered clients and shared secrets (required outside development; the last two must match the worker).

## Local Development

```bash
# Run API locally with hot reload
pnpm dev:api

# Run tests
pnpm nx test api
```

