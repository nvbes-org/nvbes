# Cross Site Scripting Prevention

This repository tracks XSS controls against the OWASP Cross Site Scripting Prevention Cheat Sheet. The source of truth is `docs/security/cross-site-scripting-prevention-controls.json`.

The active implementation uses layered controls:

- React auto-escaping by default, with raw HTML isolated behind `SafeHtml` and `VerifiedHtml`;
- shared HTML sanitization for approved HTML rendering;
- shared `SafeUrl` validation before dynamic `href` and `src` sinks;
- shared `SafeStyleElementCss` validation before custom CSS enters `<style>`;
- server-side consent screen validation for public HTTPS URLs and safe custom CSS;
- HTML escaping in account-service email templates;
- centralized CSP as defense in depth, not as the primary XSS control;
- a repository gate that scans for unsafe DOM sinks and verifies OWASP evidence.

Run:

```bash
pnpm check:cross-site-scripting-prevention
```

Any change to rendering, dynamic URLs, custom CSS, raw HTML, email templates, CSP, or frontend escape hatches must update this registry and gate.
