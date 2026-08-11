# CI/CD Security

This repository tracks CI/CD security controls against the OWASP CI/CD Security
Cheat Sheet. The machine-readable source of truth is
`docs/security/ci-cd-security-controls.json`.

The active GitHub Actions baseline is:

- read-only default `GITHUB_TOKEN` permissions;
- no `pull_request_target` or `workflow_run` execution of repository code;
- strict `bash --noprofile --norc -euo pipefail` shells for every `run` step;
- workflow concurrency and bounded job timeouts;
- `actions/checkout` with `persist-credentials: false`;
- lockfile-enforced dependency installation;
- allowlisted, full-SHA-pinned actions and allowlisted CI secrets;
- `continue-on-error` denied unless an exact workflow and step ID is explicitly
  allowlisted;
- CI/CD security self-check before dependency installation;
- generic secret-bearing release steps restricted to trusted `push` events on
  `main`;
- production email deployment restricted to the protected `production-email`
  Environment, the exact `main` commit, a signed immutable image, and the fixed
  isolated email Terraform state;
- authenticated DAST restricted to the protected staging Environment, the exact
  `main` commit selected by `github.sha`, exact preflight-validated origins, and
  fail-closed synthetic identity/access proofs before and after each scan.

Run:

```bash
pnpm check:ci-cd-security
```

New workflow actions, secrets, write permissions, deployment paths, or triggers
must be added to the registry with owner-reviewed evidence before use. The checker
scans `.github/workflows` and rejects unregistered integrations and dangerous
trust-boundary patterns.

## Email deployment trust boundary

The production email exception applies only to
`.github/workflows/deploy-email.yml`. It has no dispatch inputs, accepts only
`refs/heads/main`, checks out the exact `github.sha`, and runs through the
protected `production-email` GitHub Environment. The environment must require
independent approval and contains only email-scoped provider, state, encryption,
and delivery credentials.

The workflow consumes only
`ghcr.io/nvbes-org/nvbes-email-worker:<github.sha>`, resolves it to a digest, and
verifies the keyless signature identity of `container-release.yml`. It then
copies only the verified `linux/amd64` image into a private Scaleway Container
Registry and keyless-signs that immutable copy before passing its digest to
Terraform. Terraform is fixed to
`infrastructure/environments/email-production` and
`production/email/terraform.tfstate`; neither target nor image is accepted from
operator input. It applies saved plans, requires the SQLx migration job to reach
`succeeded`, and only then applies the runtime plan and verifies the public
shallow health endpoint.

## Account release trust boundary

The Account production exception applies only to
`.github/workflows/account-release.yml`. It has no dispatch inputs and runs through
the protected `production-account` GitHub Environment. The workflow checks out the
selected `github.ref` and
`tools/account-quality/verify-account-release-ref.mjs` requires a lightweight tag
named `account-rc-<full-commit-SHA>` that points directly to the exact
`github.sha`. Branch refs, annotated tags, short SHAs, moved tags, or a mismatching
GitHub ref API response fail closed. Repository rules must reserve creation of
`account-rc-*` to release managers and forbid updates, force-updates, and deletion.

The same immutable RC contract applies to
`.github/workflows/account-acceptance-source.yml` and
`.github/workflows/account-acceptance-ingest.yml`. The three protected
Environments allow only the `account-rc-*` deployment-tag pattern and require
independent reviewers without self-approval.

The acceptance chain is deliberately blocked:

1. `account-acceptance-source.yml` verifies the RC and CI/CD policy, then exits
   unsuccessfully at `Block candidate-controlled evidence import`. It has no
   evidence-store secret, performs no dependency installation, reads no external
   report, and publishes no artifact.
2. `account-acceptance-ingest.yml` retains the exact producer workflow, repository,
   tag, SHA, event, and successful-conclusion checks. Because the source producer
   cannot succeed, no source run ID is currently valid. If a trusted producer is
   introduced later, ingest performs the built-in cryptographic verification and
   republishes only the fixed allowlist from a fresh tree.
3. `account-release.yml` remains the only production trigger. It requires a
   successful ingest run for the same workflow, tag, and SHA, repeats the
   cryptographic verification, and runs the exact
   `scripts/release-gate.sh production` command. Dependencies and Playwright are
   installed before any downloaded evidence or trust-root material is created.

Trust-root secrets are accepted only by the ingest and production workflows. The
staging Account password is accepted only by the production workflow. The source
workflow is intentionally not authorized to receive either class of secret. These
exceptions do not authorize secret-bearing manual workflows generally.

Production promotion remains blocked until all of the following exist:

- an externally pinned importer that candidate-controlled code cannot modify;
- an ephemeral GitHub App or OIDC credential restricted to the ID-pinned private
  evidence repository;
- a confidential ACL or encryption boundary that keeps raw human and penetration
  reports out of application-repository Actions artifacts;
- bounded, traversal-safe extraction plus redirect-safe DNS/TLS connection binding;
- a signed exact-SHA import attestation;
- detached signatures and category-specific signer trust roots for individual
  human reports, including an independent pentest trust root;
- exact-SHA provenance, freshness, and digest manifests for trusted-hermetic,
  browser, k6, and DAST automation.

These are release-blocking entries in
`docs/testing/account-test-manifest.json`; they are not operator procedures that can
be bypassed locally. A local invocation of the production release gate is diagnostic
only and never constitutes a promotion.
