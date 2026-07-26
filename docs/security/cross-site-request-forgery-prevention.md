# Cross-Site Request Forgery Prevention

This repository tracks CSRF controls against the OWASP Cross-Site Request Forgery Prevention Cheat Sheet. The source of truth is `docs/security/cross-site-request-forgery-prevention-controls.json`.

The active implementation uses layered controls:

- signed double-submit CSRF cookies with an HMAC bound to the session cookie value;
- mandatory `X-CSRF-Token` checks for cookie-authenticated POST, PUT, PATCH, and DELETE requests;
- Fetch Metadata rejection for cross-site, no-cors, and subresource authenticated requests;
- Origin and Referer validation against configured first-party browser origins;
- SameSite=Strict cookies, Secure cookies outside development, host-prefixed cookie names, and HttpOnly session cookies;
- credentialed CORS only for trusted browser origins, with explicit `x-csrf-token` and `x-requested-with` preflight support;
- browser fetch wrappers that attach scoped CSRF tokens and trusted AJAX headers;
- focused Rust tests plus a repository gate for regression coverage.

Run:

```bash
pnpm check:cross-site-request-forgery-prevention
```

Any change to session cookies, CSRF tokens, browser-origin checks, CORS, login, session refresh, fetch wrappers, or authenticated mutating routes must update this registry and gate.
