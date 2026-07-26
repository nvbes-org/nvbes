# Credential Stuffing Prevention

This repository tracks credential stuffing and password spraying controls against the OWASP Credential Stuffing Prevention Cheat Sheet. The source of truth is `docs/security/credential-stuffing-prevention-controls.json`.

The active implementation uses layered controls:

- multi-step login with proof-of-work, bot guard proof, decoy handling, HTTP signal scoring, and password challenge separation;
- IP, account, and tenant throttles before credential verification;
- Redis correlation windows for IPs trying many accounts and accounts being tried from many sources;
- trusted exposed-credential screening at registration and login;
- adaptive MFA step-up when stuffing signals are present after a valid password;
- temporary blocking and tarpit delay when multiple stuffing signals stack;
- risk events and bot metrics for detected and mitigated attack volume.

Run:

```bash
pnpm check:credential-stuffing-prevention
```

Any change to login, MFA, bot guard, proof-of-work, exposed credential checks, throttling, risk scoring, or authentication telemetry must update this registry and gate.
