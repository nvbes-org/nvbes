# Start with Nvbes Dev Container

This Dev Container gives you the full Nvbes local stack without installing the project toolchain on the host.

## First Start

Run the database migrations, then start the development stack:

```bash
pnpm db:migrate
pnpm dev
```

## Useful Commands

```bash
pnpm dev:web
pnpm dev:api
pnpm check
pnpm test
pnpm verify
```

## Local URLs

- Cloud web: http://localhost:5173
- Account web: http://localhost:3001
- Account service: http://localhost:4000
- Cloud service: http://localhost:4002
- Grafana: http://localhost:13000

## Reset Local Data

```bash
pnpm dev:drive-db:reset
pnpm db:migrate
```

For the full setup notes, see `docs/development/devcontainer.md`.
