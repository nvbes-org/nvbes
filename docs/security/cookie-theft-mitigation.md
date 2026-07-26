# Cookie Theft Mitigation

This repository tracks cookie theft controls against the OWASP Cookie Theft Mitigation Cheat Sheet. The source of truth is `docs/security/cookie-theft-mitigation-controls.json`.

The active implementation stores a server-side request profile when a session is created, then validates that profile on authenticated JWT/cookie requests. The profile includes IP network, User-Agent family, Accept-Language, Accept, Accept-Encoding, Sec-Fetch headers, and browser client hints.

Detection is weighted because OWASP explicitly notes false positives and false negatives. Small differences are tolerated. Medium suspicion records risk and audit telemetry. High suspicion revokes the session and refresh tokens, then returns `session_reauthentication_required` so the browser must authenticate again and receive fresh cookies.

Bearer cookie hardening remains mandatory: session cookies are `HttpOnly`, `SameSite=Strict`, `Secure` outside development, and `__Host-` scoped when secure. This mitigation complements those attributes; it does not replace XSS, CSRF, CSP, bot, and authentication controls.

Run:

```bash
pnpm check:cookie-theft-mitigation
```

Any change to session creation, JWT middleware, token refresh, cookie attributes, request headers, risk scoring, or reauthentication behavior must update the registry and this gate.
