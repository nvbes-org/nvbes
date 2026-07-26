# Authentication Protection

This repository tracks authentication controls against the OWASP Authentication Cheat Sheet. The source of truth is `docs/security/authentication-controls.json`.

The registry maps OWASP requirements to concrete controls and evidence strings in code. It covers identity identifiers, password policy, breached credential handling, password hashing and verification, recovery/change flows, cookies and transport hardening, reauthentication, generic errors, automated attack defenses, MFA/WebAuthn, OAuth/OIDC/JWT, and authentication logging.

## Check

Run:

```bash
pnpm check:authentication
```

The check fails when:

- a requirement or control ID is duplicated or malformed;
- a requirement has no controls;
- a control has no evidence;
- an evidence file is missing;
- a required evidence string is no longer present.

The root `pnpm check` command also runs this validation.

## Current Coverage

| ID              | Requirement                                         |
| --------------- | --------------------------------------------------- |
| `AUTHN_REQ_001` | Identity identifiers and username handling          |
| `AUTHN_REQ_002` | Password strength and breached credential rejection |
| `AUTHN_REQ_003` | Password storage and comparison                     |
| `AUTHN_REQ_004` | Password recovery and change                        |
| `AUTHN_REQ_005` | Transport and cookie protections                    |
| `AUTHN_REQ_006` | Reauthentication and step-up                        |
| `AUTHN_REQ_007` | Generic errors and enumeration resistance           |
| `AUTHN_REQ_008` | Automated attack protections                        |
| `AUTHN_REQ_009` | MFA and FIDO/WebAuthn                               |
| `AUTHN_REQ_010` | Token, OAuth, and OIDC authentication               |
| `AUTHN_REQ_011` | Logging and monitoring                              |

## Maintenance Rules

Every auth-significant change must update this registry when it adds, removes, or changes an authentication control. Prefer evidence in executable checks, tests, middleware, cryptographic helpers, migrations, or narrowly scoped production modules.

Accepted gaps must not be hidden by weakening evidence. Add a new requirement/control with explicit status and product/security tracking context if a control is intentionally deferred.
