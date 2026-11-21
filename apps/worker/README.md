# NVBES Worker - Unified Background Worker

> **Référence d'Architecture (ADR 0013)** : [0013-modular-monolith-api-and-unified-worker.md](../../docs/adr/0013-modular-monolith-api-and-unified-worker.md).

Central background worker handling asynchronous processing for NVBES, consolidating legacy worker operations (`billing-worker`, `email-worker`) into a single serverless container:

- **Transactional Outbox Dispatcher**: Sweeps outbox events directly from PostgreSQL (Scaleway TEM transactional email, Stripe billing receipts & payment failure notifications).
- **GDPR 30-Day Purges**: Executes periodic privacy jobs (exports expiry, account closures after 30-day grace period).
- **Scale-to-Zero & Serverless**: Zero-CLI container entrypoint, scale-to-zero connection pool (`min_connections = 0`, 30s `idle_timeout`), graceful shutdown on `SIGTERM` / `SIGINT`.
- **Health Probes**: Embedded HTTP server for container liveness and readiness (`/healthz`, `/readyz`) on `$PORT` (default 8081).

## Environment Variables

| Variable | Default | Description |
| -------- | ------- | ----------- |
| `PORT` | `8081` | HTTP port for `/healthz` and `/readyz` probes |
| `DATABASE_URL` | `postgres://postgres:postgres@localhost:5432/nvbes_dev` | PostgreSQL connection string |
| `POLL_INTERVAL_SECS` | `5` | Polling interval for transactional outbox dispatch |
| `PRIVACY_INTERVAL_SECS` | `60` | Interval for running GDPR retention & privacy jobs |

## Local Development

```bash
# Run worker locally with hot reload
pnpm dev:worker

# Run tests
pnpm nx test worker
```

