# Contributing

Guidelines for contributing to the nvbes monorepo.

## Before opening a change

- Open an issue for broad product, architecture or persistence changes.
- Use a fork or a short-lived branch; never push directly to `main`.
- Do not include credentials, production data, personal data or copied code
  whose provenance and licence are unknown.
- Contributions are licensed under the licence of the component they modify,
  as described in `docs/legal/open-source-licensing.md`.

## Contribution Rules

- Follow the architectural boundaries defined in `AGENTS.md` and Nx configuration.
- Product services must remain loosely coupled through well-defined API / event contracts.
- Do not commit secrets, credentials, or `.env` files.
- Keep files short, flat, and modular (< 300 lines recommended).
- Keep changes focused and document migrations, recurring cost and rollback.

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

Run additional targeted tests for every changed behaviour. A skipped check must
be explained in the pull request.

## Pull requests

- Explain the problem, the delivered outcome and the evidence collected.
- Keep generated files deterministic and update contracts with their producers.
- Confirm that third-party code and assets retain compatible attribution.
- Resolve review threads and keep the branch current with `main`.
- Maintainers may request an ADR for security boundaries, persistent formats,
  public contracts or irreversible operational choices.

Security findings follow `SECURITY.md`, not the public issue tracker.
