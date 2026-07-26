# Authorization Testing Automation

This repository tracks automated authorization matrix coverage against the OWASP Authorization Testing Automation Cheat Sheet. The source of truth is `docs/security/authorization-testing-automation-matrix.json`.

The matrix is a pivot file: it lists points of view, protected services, allowed and denied roles, expected status codes, test payloads, and evidence strings that bind the matrix to production authorization code and regression tests.

## Check

Run:

```bash
pnpm check:authorization-testing-automation
```

The check fails when:

- a point of view or service ID is duplicated or malformed;
- a service does not classify every point of view as allowed or denied;
- a service marks the same point of view as both allowed and denied;
- allowed or denied status codes are not declared in the matrix defaults;
- a service has no matching `servicesTesting` payload fixture;
- an evidence file is missing or a required evidence string is no longer present.

The root `pnpm check` command also runs this validation.

## Matrix Model

| Section           | Purpose                                                       |
| ----------------- | ------------------------------------------------------------- |
| `pointsOfView`    | Logical roles and auth contexts used by automated tests       |
| `services`        | Feature/resource/action combinations and expected decisions   |
| `servicesTesting` | Path parameters and payload fixtures consumed by test runners |
| `evidence`        | Code/test markers proving the matrix remains wired to code    |

## Maintenance Rules

Every authorization-sensitive endpoint must be represented in this matrix or in a more specific generated matrix before release. New roles, actions, token profiles, resource identifiers, tenant scopes, or request payloads must update both `services` and `servicesTesting`.

Do not leave a point of view implicit. Every service must explicitly classify every point of view as allowed or denied so a newly introduced role cannot fall through untested.
