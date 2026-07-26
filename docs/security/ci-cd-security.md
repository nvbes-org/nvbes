# CI/CD Security

This repository tracks CI/CD security controls against the OWASP CI/CD Security Cheat Sheet. The source of truth is `docs/security/ci-cd-security-controls.json`.

The active GitHub Actions baseline is:

- read-only default `GITHUB_TOKEN` permissions;
- no `pull_request_target` or `workflow_run` execution of repository code;
- strict `bash --noprofile --norc -euo pipefail` shell for all `run` steps;
- job `timeout-minutes` and workflow `concurrency`;
- `actions/checkout` with `persist-credentials: false`;
- dependency installation with lockfile enforcement;
- allowlisted third-party actions and CI secrets;
- secret-bearing release build step restricted to trusted `push` on `main`;
- CI/CD security self-check before dependency installation.

Run:

```bash
pnpm check:ci-cd-security
```

New workflow actions, secrets, write permissions, deployment paths, or workflow triggers must be added to the registry with owner-reviewed evidence before use. Only explicitly allowed entries in the registry may pass. The checker scans `.github/workflows` and fails on unregistered integrations or dangerous trust-boundary patterns.
