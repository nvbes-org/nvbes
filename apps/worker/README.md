# NVBES Worker - Unified Background Worker

Central background worker handling asynchronous processing for NVBES:
- **Transactional Outbox Dispatcher**: Dispatches outbox events (Scaleway TEM transactional email, Stripe billing notifications).
- **GDPR 30-Day Purges**: Executes periodic privacy jobs (exports expiry, account closures after 30-day grace period).
- **Scale-to-Zero & Serverless**: Scale-to-zero pool (`min_connections = 0`), graceful shutdown on `SIGTERM` / `SIGINT`.
- **Health Probes**: Embedded HTTP server for container liveness and readiness (`/healthz`, `/readyz`).
