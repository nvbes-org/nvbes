# Bot Management and Anti-Automation

This repository tracks bot-management and anti-automation controls against the OWASP Bot Management and Anti-Automation Cheat Sheet. The source of truth is `docs/security/bot-management-controls.json`.

These controls protect critical properties but do not authorize autonomous
irreversible moderation. Trust/Risk starts in shadow mode, and the solo operator
reviews ambiguous abuse decisions manually under the global EUR 30 budget.

The registry maps OWASP guidance to concrete controls and evidence strings in code. It covers threat modeling by sensitive flow, layered defenses, multi-key rate limiting, privacy-aware device and behavior signals, PoW as a CAPTCHA alternative, honeypots, public API anti-automation, graduated responses, monitoring, and credential-stuffing defenses.

## Check

Run:

```bash
pnpm check:bot-management
```

The check fails when:

- a requirement or control ID is duplicated or malformed;
- a requirement has no controls;
- a control has no evidence;
- an evidence file is missing;
- a required evidence string is no longer present.

The root `pnpm check` command also runs this validation.

## Current Coverage

| ID            | Requirement                                  |
| ------------- | -------------------------------------------- |
| `BOT_REQ_001` | Threat-model sensitive automated abuse flows |
| `BOT_REQ_002` | Layered defense architecture                 |
| `BOT_REQ_003` | Rate limiting and quotas                     |
| `BOT_REQ_004` | Privacy-aware device and behavior signals    |
| `BOT_REQ_005` | CAPTCHA alternatives and proof-of-work       |
| `BOT_REQ_006` | Honeypots and tarpits                        |
| `BOT_REQ_007` | Public API anti-automation                   |
| `BOT_REQ_008` | Graduated response strategy                  |
| `BOT_REQ_009` | Logging and monitoring                       |
| `BOT_REQ_010` | Credential stuffing defenses                 |

## Maintenance Rules

Every auth, signup, public API, rate-limit, risk-signal, or anti-automation change must update this registry when it adds, removes, weakens, or relocates a control.

Do not weaken evidence to make the gate pass. Replace removed controls with equivalent or stronger evidence in the same change.
