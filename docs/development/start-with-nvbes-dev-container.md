# Start with Nvbes Dev Container

This Dev Container gives you the full Nvbes local stack without installing the project toolchain on the host.

## First Start

Start the complete development stack, including its local infrastructure:

```bash
pnpm dev
```

## Useful Commands

```bash
pnpm dev:email-worker
pnpm dev:trust-risk-service
pnpm dev:identity-service
pnpm dev:account-service
pnpm dev:billing-service
pnpm check
pnpm test
pnpm verify
```

## Local URLs

- Email HTTP/gRPC: http://localhost:3040
- Trust/Risk HTTP/gRPC: http://localhost:3050
- Identity: http://localhost:3060
- Account: http://localhost:3070
- Billing: http://localhost:3080
- Mailpit: http://localhost:8025
- Grafana: http://localhost:13000

For the full setup notes, see `docs/development/devcontainer.md`.
