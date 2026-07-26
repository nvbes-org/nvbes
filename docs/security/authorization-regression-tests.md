# Authorization Regression Testing

This repository tracks authorization regression suites against the OWASP Authorization Regression Testing Cheat Sheet. The source of truth is `docs/security/authorization-regression-tests.json`.

The registry maps OWASP regression guidance to concrete test families and evidence strings in code. It covers actor-resource-action matrices, horizontal IDOR replay, vertical role demotion, tenant and workspace isolation, token scope/session switching, contract-driven validation, fail-closed denied outcomes, and CI/CD gating.

## Check

Run:

```bash
pnpm check:authorization-regression
```

The check fails when:

- a command, suite, or evidence ID is duplicated or malformed;
- a suite has no evidence;
- an evidence file is missing;
- a required evidence string is no longer present;
- a registered command is not exposed from `package.json` when it is a pnpm script.

The root `pnpm check` command also runs this validation.

## Current Suites

| ID              | Suite                                        |
| --------------- | -------------------------------------------- |
| `AUTHZ_REG_001` | Actor-resource-action matrix                 |
| `AUTHZ_REG_002` | Horizontal escalation and IDOR replay        |
| `AUTHZ_REG_003` | Vertical escalation role demotion            |
| `AUTHZ_REG_004` | Tenant and workspace isolation               |
| `AUTHZ_REG_005` | Token scope and session-switching regression |
| `AUTHZ_REG_006` | Contract-driven authorization validation     |
| `AUTHZ_REG_007` | Denied outcome and fail-closed regression    |
| `AUTHZ_REG_008` | CI/CD authorization gate                     |

## Maintenance Rules

Every authorization-sensitive change must keep at least one regression suite proving that the changed boundary fails closed. New routes, roles, actions, token scopes, resource identifiers, tenant boundaries, or policy-decision contracts must either reuse an existing suite or add a new suite entry.

Do not weaken evidence to make the gate pass. If a regression test is intentionally removed, replace it with an equivalent or stronger negative test and update this registry in the same change.
