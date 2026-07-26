# Authorization Protection

This repository tracks authorization controls against the OWASP Authorization Cheat Sheet. The source of truth is `docs/security/authorization-controls.json`.

The registry maps OWASP requirements to concrete controls and evidence strings in code. It covers least privilege, deny-by-default policy behavior, per-request server-side authorization, IDOR resistance, tenant and workspace scoping, token scopes, step-up obligations, privileged backoffice actions, tenant administration, row-level database context, denied-decision audit logging, and enterprise policy decision consistency.

## Check

Run:

```bash
pnpm check:authorization
```

The check fails when:

- a requirement or control ID is duplicated or malformed;
- a requirement has no controls;
- a control has no evidence;
- an evidence file is missing;
- a required evidence string is no longer present.

The root `pnpm check` command also runs this validation.

## Current Coverage

| ID              | Requirement                                        |
| --------------- | -------------------------------------------------- |
| `AUTHZ_REQ_001` | Least privilege and deny by default                |
| `AUTHZ_REQ_002` | Server-side authorization on every request         |
| `AUTHZ_REQ_003` | Resource-context authorization and IDOR resistance |
| `AUTHZ_REQ_004` | Tenant, workspace, and delegation boundaries       |
| `AUTHZ_REQ_005` | API token scope authorization                      |
| `AUTHZ_REQ_006` | Sensitive action step-up authorization             |
| `AUTHZ_REQ_007` | Privileged backoffice authorization                |
| `AUTHZ_REQ_008` | Tenant administration authorization                |
| `AUTHZ_REQ_009` | Database row-level authorization context           |
| `AUTHZ_REQ_010` | Denied authorization logging and monitoring        |
| `AUTHZ_REQ_011` | Policy decision consistency                        |

## Maintenance Rules

Every authorization-significant change must update this registry when it adds, removes, or changes an access-control decision, policy boundary, privileged role, object-scope lookup, token-scope rule, or denied-decision audit event.

Accepted gaps must not be hidden by weakening evidence. Add a new requirement/control with explicit status and product/security tracking context if a control is intentionally deferred.
