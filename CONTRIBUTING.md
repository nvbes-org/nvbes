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

## Commit messages

Commits and pull request titles follow Conventional Commits:

```text
feat(identity): add a sign-in method
fix(billing): correct the invoice total
chore(deps): update dependencies
```

Use a lowercase type: `build`, `chore`, `ci`, `docs`, `feat`, `fix`, `perf`,
`refactor`, `revert`, `style`, or `test`. Scopes are optional and unrestricted.
Headers must not exceed 100 characters. Mark breaking changes with `!` before
the colon or a `BREAKING CHANGE:` footer.
Default message exemptions are disabled, so merge, revert and version-only
messages must also follow the convention.

After installing dependencies, run `pnpm exec lefthook install` to enable the
local `commit-msg` check. To check an existing commit, run
`pnpm exec commitlint --last --verbose`. CI validates all PR commits and the PR
title, including title edits, using the same configuration. The title must also
describe the final change because it becomes the squash commit subject.

This check validates message structure; it does not determine release readiness
or publish packages. The workflow uses no secrets or paid services and has a
five-minute timeout. Removing the config, hook, workflow and dev dependencies
reverts the integration.

## Pull requests

- Explain the problem, the delivered outcome and the evidence collected.
- Keep generated files deterministic and update contracts with their producers.
- Confirm that third-party code and assets retain compatible attribution.
- Resolve review threads and keep the branch current with `main`.
- Maintainers may request an ADR for security boundaries, persistent formats,
  public contracts or irreversible operational choices.

Security findings follow `SECURITY.md`, not the public issue tracker.
