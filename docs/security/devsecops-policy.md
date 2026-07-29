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
- property tests and bounded fuzz smoke tests for DPoP/JWT, OAuth and uploads;
- the existing repository security-control registries.

An exception must be narrow, owned, documented with exploitability analysis and
time-bounded. Disabling a job or using `continue-on-error` is not an exception.
Yanked transitive crates are warnings until their upstream dependency permits a
non-yanked replacement; security advisories remain blocking. Workspace-local
path dependencies are exempt from registry version pinning.

The scheduled DAST job becomes active after configuring the repository
variables `DAST_ACCOUNT_URL`, `DAST_BACKOFFICE_URL` and `DAST_OPENAPI_URL`.
Manual dispatch requires the same three HTTPS targets and is available for
staging verification before a release.

## Remediation SLA

The timer starts when a vulnerability is detected or privately disclosed.

| Severity | Remediation target | Required handling |
|---|---:|---|
| Critical | 24 hours; 72 hours maximum for a documented non-exploitable case | Incident owner, immediate containment, release or compensating control |
| High | 7 calendar days | Security owner and tracked remediation |
| Medium | Planned in the next normal delivery cycle, maximum 30 days | Backlog item with owner and due date |
| Low | Risk-based | Track when relevant to an exposed path |

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
- two approvals;
- CODEOWNERS approval for identity, cryptography and infrastructure;
- dismissal of stale approvals;
- resolution of review conversations;
- signed commits and linear history;
- the CI and security status checks;
- administrator enforcement and no force pushes or deletion.

GitHub currently reports that branch protection/rulesets require a plan upgrade
for this private repository. `.github/CODEOWNERS` and
`.github/rulesets/main-branch-policy.json` are the versioned desired state to
apply as soon as the repository plan supports enforcement.

## NIST SSDF 1.1 mapping

| SSDF practice | Repository implementation |
|---|---|
| PO.1, PO.3 | CODEOWNERS, remediation SLA, security control registries |
| PS.1, PS.2 | Least-privilege workflows, full-SHA action pins, protected release identity |
| PW.4, PW.7 | CodeQL, property tests, fuzzing, DAST and review gates |
| PW.9 | OSV, cargo-deny, Gitleaks and Trivy |
| PS.3 | CycloneDX SBOM, provenance, digest-only release evidence and Cosign |
| RV.1, RV.2, RV.3 | Scheduled scanning, severity SLA and documented exception lifecycle |
