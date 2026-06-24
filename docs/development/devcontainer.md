# Dev Container Development Environment

This repository supports a full-stack Dev Container environment for local or remote development.

## Host Requirements

Use one of:

- Docker Desktop or a Docker-compatible runtime with Dev Containers support.
- Coder, DevPod, Codespaces, or another runner that supports the Dev Container specification.

No local Node.js, pnpm, Rust, PostgreSQL, Redis, OpenTofu, or SQLx CLI installation is required inside this workflow.

## Included Tooling

The `workspace` container includes:

- Node.js 25.8.0
- pnpm 11.1.3
- Rust stable with `rustfmt` and `clippy`
- `sqlx-cli` with PostgreSQL support
- OpenTofu
- Go, Python, PostgreSQL client tools, Redis tools, ShellCheck, `jq`, `curl`, and build tooling
- Native Rust FFI dependencies for SAML/XML security: `libxml2`, `xmlsec1`, OpenSSL, `pkg-config`, and `libclang`

## Included Services

The Dev Container Docker Compose stack starts:

- PostgreSQL 17 on host port `localhost:15432`
- Redis Stack on host port `localhost:16379`
- RedisInsight on host port `localhost:18001`
- Prometheus on host port `localhost:19090`
- Tempo on host port `localhost:13200`
- Tempo OTLP gRPC on host port `localhost:14317`
- Tempo OTLP HTTP on host port `localhost:14318`
- Loki on host port `localhost:13100`
- Pyroscope on host port `localhost:14040`
- Grafana on host port `localhost:13000`

PostgreSQL initializes both local databases:

- `nvbes`
- `nvbes_drive`

## First Start

Open the repository in the Dev Container. The `post-create` step installs workspace dependencies and registers Lefthook.

Then run:

```bash
pnpm db:migrate
pnpm dev
```

Application URLs:

- Drive web: `http://localhost:5173`
- Identity web: `http://localhost:3001`
- Identity API: `http://localhost:4000`
- Drive API: `http://localhost:4002`
- Grafana: `http://localhost:13000`

## Environment File

If `.env` does not exist, `.devcontainer/post-create.sh` creates it from `.env.example` and adjusts service hostnames for container networking:

- PostgreSQL host: `postgres`
- Redis host: `redis`

The script does not overwrite an existing `.env`.

When running inside the Dev Container, Compose sets `NVBES_DEVCONTAINER=true`. The shared environment loader then overrides database and Redis endpoints for container networking, so an existing host-oriented `.env` with `localhost` does not break `pnpm dev`.

## Resetting Local Data

Inside the Dev Container:

```bash
pnpm dev:drive-db:reset
pnpm db:migrate
```

To remove all container volumes from the host, stop the Dev Container stack and remove the `nvbes_*` Docker volumes.

## Coder

Coder should consume the same `.devcontainer/devcontainer.json` rather than maintaining a separate development image. This keeps local, remote, and agent workspaces aligned.
