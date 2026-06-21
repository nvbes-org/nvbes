# Project Memory

nvbes is a Rust/Axum and TypeScript/React monorepo managed with pnpm workspaces,
Nx, and a Cargo workspace.

## Stable Boundaries

- `apps/identity-api` and `apps/drive-api` are product APIs.
- `apps/*-web` are React frontends.
- `apps/*-worker` are async workers.
- `libs/rust/*` contains shared Rust primitives and domain libraries.
- `libs/ts/*` contains SDKs, clients, UI, and web runtime packages.

## Durable Rules

- Rust files use flat dot-qualified filenames in `src/`.
- TypeScript must not use `any`.
- V0 code should not accept temporary debt in touched areas.
- Agent instructions should flow from `docs/agent/*`.
