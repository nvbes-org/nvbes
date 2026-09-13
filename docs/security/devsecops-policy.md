# DevSecOps and vulnerability management

This policy implements the repository security baseline using NIST SSDF 1.1,
OWASP supply-chain guidance and immutable OCI release evidence.

## Merge gates

Every pull request is expected to pass:

- OSV-Scanner over every committed Rust and JavaScript lockfile;
- `cargo deny check` for advisories, licenses, bans and sources;
- CodeQL for JavaScript, TypeScript and GitHub Actions;
- Gitleaks over the complete Git history;
- Trivy filesystem, secret and infrastructure-as-code scanning;
- property-based tests for critical HTTP headers, token validation and security parsers;
- the existing repository security-control registries.

An exception must be narrow, owned, documented with exploitability analysis and
time-bounded. Disabling a job or using `continue-on-error` is not an exception.
Yanked transitive crates are warnings until their upstream dependency permits a
non-yanked replacement; security advisories remain blocking. Workspace-local
path dependencies are exempt from registry version pinning.

The scheduled DAST job is blocking and requires the repository variables
`DAST_ACCOUNT_URL`, `DAST_API_URL`, `DAST_BACKOFFICE_URL`, `DAST_OPENAPI_URL` and
`DAST_ALLOWED_ORIGINS`, the synthetic fixture variables
`DAST_EXPECTED_SUBJECT_ID` and `DAST_EXPECTED_TENANT_ID`, plus the staging-only secrets
`DAST_ACCOUNT_SESSION_COOKIE`, `DAST_API_AUTHORIZATION` and
`DAST_BACKOFFICE_SESSION_COOKIE`. Targets must belong to the exact HTTPS
staging-origin allowlist, resolve only to public addresses and must never
redirect to production or private infrastructure. Manual dispatch uses the same
controls for pre-release verification. The API scan always overrides every
server declared by the OpenAPI document with the independently validated
staging API host. Immediately before and after every ZAP scan, the Account and
API credentials must prove their synthetic subject/tenant identity, while the
Backoffice credential must prove authorization to read that exact fixture. The
six redacted proof artifacts and their limits are documented in
[`dast-authenticated-staging.md`](dast-authenticated-staging.md).

## Remediation SLA

The timer starts when a vulnerability is detected or privately disclosed.

| Severity |                                               Remediation target | Required handling                                                      |
| -------- | ---------------------------------------------------------------: | ---------------------------------------------------------------------- |
| Critical | 24 hours; 72 hours maximum for a documented non-exploitable case | Incident owner, immediate containment, release or compensating control |
| High     |                                                  7 calendar days | Security owner and tracked remediation                                 |
| Medium   |       Planned in the next normal delivery cycle, maximum 30 days | Backlog item with owner and due date                                   |
| Low      |                                                       Risk-based | Track when relevant to an exposed path                                 |

Expired exceptions fail the security gate. Internet-exposed authentication,
authorization, cryptography and tenant-isolation findings are never downgraded
solely because exploitation has not yet been observed.

## Release evidence

Each release image is:

1. built from a tag using pinned actions and pinned base-image digests;
2. referenced by immutable OCI digest;
3. scanned by Trivy before release completion;
4. accompanied by a CycloneDX SBOM;
5. covered by GitHub/Sigstore build provenance;
6. signed keylessly with Cosign using the release workflow OIDC identity;
7. verified before the workflow succeeds.

Kubernetes admission policy accepts only images signed by
`.github/workflows/container-release.yml` from this repository. It also requires
non-root execution, a read-only root filesystem, disabled privilege escalation,
all Linux capabilities dropped and the runtime-default seccomp profile.

## Branch and review policy

`main` must require:

- pull requests rather than direct pushes;
- maintainer approval for contributions externes ;
- CODEOWNERS pour router les revues sensibles sans bloquer les PR du mainteneur solo ;
- dismissal of stale approvals when a review is present;
- resolution of review conversations;
- signed commits and linear history;
- the CI and security status checks;
- administrator enforcement and no force pushes or deletion.

GitHub reports that branch protection and rulesets are unavailable while the
repository remains private on its current plan. `.github/CODEOWNERS` and
`.github/rulesets/main-branch-policy.json` are the versioned desired state to
apply immediately after the repository becomes public. The zero mandatory
approval count is intentional for a solo maintainer; external contributions
remain reviewed by the maintainer before merge.

## NIST SSDF 1.1 mapping

| SSDF practice    | Repository implementation                                                   |
| ---------------- | --------------------------------------------------------------------------- |
| PO.1, PO.3       | CODEOWNERS, remediation SLA, security control registries                    |
| PS.1, PS.2       | Least-privilege workflows, full-SHA action pins, protected release identity |
| PW.4, PW.7       | CodeQL, property tests (proptest), DAST and review gates                    |
| PW.9             | OSV, cargo-deny, Gitleaks and Trivy                                         |
| PS.3             | CycloneDX SBOM, provenance, digest-only release evidence and Cosign         |
| RV.1, RV.2, RV.3 | Scheduled scanning, severity SLA and documented exception lifecycle         |
