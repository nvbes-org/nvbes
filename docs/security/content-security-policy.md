# Content Security Policy

This repository tracks CSP controls against the OWASP Content Security Policy Cheat Sheet. The source of truth is `docs/security/content-security-policy-controls.json`.

The active implementation has two baselines:

- web applications build their `Content-Security-Policy` header through `libs/ts/web-runtime/src/csp.ts`;
- Rust API responses receive a deny-by-default CSP from `libs/rust/core/src/security.headers.rs`.

The web CSP blocks object embedding, event-handler script attributes, unexpected form targets, and cross-origin framing. Production script policies do not include `unsafe-inline` or `unsafe-eval`. Inline styles remain enabled because the React component stack and tenant theming generate style elements and attributes at runtime; this exception does not relax script execution. Product-specific third-party origins, such as Stripe or telemetry endpoints, must be passed explicitly through the shared builder.

The policy does not use `upgrade-insecure-requests`. Production transport security is enforced by HTTPS and HSTS, while omitting the directive keeps HTTP object-storage URLs usable in local production previews instead of rewriting them to unsupported HTTPS endpoints.

The API CSP is intentionally stricter than the web policy: `default-src 'none'` with explicit denial for scripts, styles, images, fonts, workers, frames, objects, media, and manifests. This keeps JSON/API responses non-executable while still reporting policy violations.

Run:

```bash
pnpm check:content-security-policy
```

New scripts, frames, remote styles, fonts, image origins, connect endpoints, CSP reporting endpoints, or header-delivery changes must update the registry and the shared CSP builder. Do not add local CSP string literals in Vite configs; use `buildWebCsp` so every product keeps the same baseline.
