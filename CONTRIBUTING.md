# Contributing

Guidelines for contributing to the nvbes monorepo.

## Contribution Rules

- Follow the architectural boundaries defined in `AGENTS.md` and Nx configuration.
- Product services must remain loosely coupled through well-defined API / event contracts.
- Do not commit secrets, credentials, or `.env` files.
- Keep files short, flat, and modular (< 300 lines recommended).

## Local Checks

Run the verification suite before submitting changes:

```bash
pnpm check
pnpm format:check
pnpm lint
```

Rust changes require:

```bash
cargo check --workspace
```

