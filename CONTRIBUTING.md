# Contributing

nvbes uses a private source-of-truth monorepo and exports an OSS repository from an allowlist.

## Contribution Rules

- Product features belong in OSS-compatible packages.
- OSS code must not import Cloud or Internal code.
- External integrations must go through provider interfaces and adapters.
- Public docs belong under `docs/oss` in the private monorepo and become `docs/` in the OSS export.
- Do not commit secrets, `.env` files, production runbooks or private architecture plans.

## Local Checks

Run the relevant checks before submitting changes:

```bash
pnpm check:oss-boundaries
pnpm check
```

Rust changes still require:

```bash
cargo check --workspace
```
