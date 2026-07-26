# Abuse Case Protection

This repository tracks abuse cases as security requirements, following the OWASP Abuse Case approach: identify concrete attacks, rate business or technical risk, select countermeasures, decide whether each case is addressed or accepted, and validate that the protection remains in place.

The source of truth is `docs/security/abuse-cases.json`. It replaces a spreadsheet with a reviewable, machine-checkable registry containing:

- `features`: product surfaces included in the review.
- `countermeasures`: defensive controls and their implementation location.
- `abuseCases`: attacker scenarios with unique IDs, risk, handling decision, countermeasure IDs, and validation evidence.

## Check

Run:

```bash
pnpm check:abuse-cases
```

The check fails when:

- an abuse case ID, feature ID, or countermeasure ID is duplicated or malformed;
- a `to_address` case has no countermeasure or no validation;
- an `accepted` case has no acceptance rationale;
- validation evidence points to a missing file;
- a required evidence string is no longer present.

The root `pnpm check` command also runs this validation.

## Current Coverage

| ID               | Feature                             | Risk     | Abuse scenario                                                                          |
| ---------------- | ----------------------------------- | -------- | --------------------------------------------------------------------------------------- |
| `ABUSE_CASE_001` | Account authentication              | high     | Credential stuffing, spraying, registration abuse, and bot-driven auth probes           |
| `ABUSE_CASE_002` | Account authentication              | high     | MFA brute force, recovery abuse, and repeated failed step-up attempts                   |
| `ABUSE_CASE_003` | Tenant and workspace authorization  | critical | Tenant, organization, workspace, or object identifier manipulation                      |
| `ABUSE_CASE_004` | Browser mutating requests           | high     | Forged cookie-authenticated mutations and weak AJAX handling                            |
| `ABUSE_CASE_005` | Web UI rendering                    | high     | Clickjacking and missing browser hardening headers                                      |
| `ABUSE_CASE_006` | Web UI rendering                    | high     | DOM XSS through unsafe AJAX data, reviewed HTML bypasses, or JSONP                      |
| `ABUSE_CASE_007` | Cloud public access                 | high     | Public API tampering, replay, and automated public share download abuse                 |
| `ABUSE_CASE_008` | Billing and developer webhooks      | high     | Fake, tampered, stale, or duplicate webhook payloads                                    |
| `ABUSE_CASE_009` | Backoffice operations               | critical | Unauthorized or irreversible privileged backoffice mutations                            |
| `ABUSE_CASE_010` | Audit, monitoring, and supply chain | critical | Audit evidence alteration or deletion                                                   |
| `ABUSE_CASE_011` | Audit, monitoring, and supply chain | high     | Secret leaks, vulnerable dependencies, missing SBOM evidence, or weak container hygiene |

## Maintenance Rules

Every new security-significant feature must add or update at least one abuse case before release. Prefer automated validation whenever the defense is implemented in code, migration SQL, configuration, or repository tooling. Use manual validation only when the evidence is external, such as a penetration test, architecture review, or operational runbook sign-off.

Accepted risk must be explicit: set `decision` to `accepted`, add an `acceptance` rationale, and record who accepted the risk and until when in the linked product or security tracking artifact.
