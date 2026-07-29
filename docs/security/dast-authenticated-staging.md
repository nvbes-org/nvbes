# Authenticated staging DAST

The scan job is bound to the protected GitHub Environment
`account-dast-staging` and refuses every ref except `refs/heads/main`. Store the
three long-lived synthetic credentials only in that Environment, with required
reviewers and a main-only deployment branch policy; repository-level secrets
are not an acceptable substitute.

The scheduled DAST workflow fails before ZAP starts unless the Account and API
credentials are bound to the configured synthetic subject and tenant, and the
Backoffice credential is authorized to read that exact fixture. Every scan is
also followed by the same proof. An expired, anonymous, redirected, or
cross-tenant response cannot produce a green authenticated-scan claim.

## Required staging configuration

Repository variables:

- `DAST_ACCOUNT_URL`, `DAST_API_URL`, `DAST_BACKOFFICE_URL` and
  `DAST_OPENAPI_URL`: HTTPS staging targets;
- `DAST_ALLOWED_ORIGINS`: comma-separated, exact bare HTTPS staging origins;
- `DAST_PRODUCTION_ORIGINS`: optional additional production origins that must
  never be scanned;
- `DAST_EXPECTED_SUBJECT_ID`: UUID of the dedicated synthetic test principal;
- `DAST_EXPECTED_TENANT_ID`: UUID of its dedicated synthetic tenant.

Environment secrets:

- `DAST_ACCOUNT_SESSION_COOKIE`: complete Account `Cookie` header value;
- `DAST_API_AUTHORIZATION`: complete `Bearer` authorization value;
- `DAST_BACKOFFICE_SESSION_COOKIE`: complete Backoffice `Cookie` header value.

The subject and tenant IDs are fixture identifiers, not credentials. The
fixture must contain no real personal data, must not exist in production, and
must have only the permissions needed to reach the read-only preflight routes
and the intended DAST scope.

## Fail-closed authentication proof

`tools/account-quality/check-dast-authentication.mjs` performs these exact
read-only requests:

| Surface | Protected request | Exact assertions |
|---|---|---|
| Account | `GET /auth/me` with the Account cookie | HTTP 200, `user.id` and `current_tenant_id` match the fixture |
| Account API | `GET /oauth/userinfo` with the bearer | HTTP 200, `sub` and `tenant_id` match the fixture |
| Backoffice | `GET /admin/users/{syntheticSubject}` with the Backoffice cookie | HTTP 200, returned `principal_id` and `tenant_id` match the requested fixture |

The Backoffice request proves authorization to read the exact synthetic
fixture. Because that resource is not an actor-introspection endpoint, it does
not prove that the Backoffice operator has the same subject identifier.

Every endpoint is rebuilt from a separately validated origin, resolved to
public IP addresses before credentials are sent, and requested with
`redirect: manual`. Redirects, including redirects to login, production, or a
private address, therefore fail with a non-200 status and are never followed
with credentials.

Credential values are passed through step-scoped environment variables. They
are never command-line arguments and the checker never prints response bodies
or request headers. On success each invocation writes one of six evidence
files: `auth-preflight-{account,api,backoffice}-{before,after}.json`. They
contain statuses, route templates and assertion names only. The ZAP report
artifact is valid authenticated-scan evidence only when both proofs surrounding
the scan are present and the workflow succeeded.
The complete ZAP artifact remains security-sensitive staging evidence, is
retained for 30 days, and must not be copied into a public issue or release.

## ZAP coverage and limits

The Account and Backoffice baseline scans enable ZAP's client spider in
addition to the traditional spider. The baseline scan is passive: it discovers
and observes traffic but does not actively attack the web surfaces. The client
spider improves discovery for JavaScript applications, but it is not proof
that every SPA state, feature-flag branch, modal, or client-side route was
visited.

The API scan actively tests operations described by the Account OpenAPI
document and overrides its servers with the independently validated staging
API host. It does not cover undocumented endpoints, gRPC boundaries,
background worker processing, or third-party integrations.

Static cookie and bearer values are not refreshed by this workflow. The proofs
bound each scan with valid protected requests immediately before and after it;
they do not prove that every individual ZAP request remained authenticated.
Review ZAP reports for authentication failures and provision credentials whose
controlled lifetime covers the job timeout. WebAuthn/passkey ceremonies, MFA
transitions, OAuth redirect state, destructive operations, multi-account
switching, and cross-tenant authorization require the deterministic
E2E/security suites and human penetration testing in addition to DAST.

References:

- [ZAP baseline scan](https://www.zaproxy.org/docs/docker/baseline-scan/)
- [ZAP client spider](https://www.zaproxy.org/docs/desktop/addons/client-side-integration/spider/)
- [ZAP API scan](https://www.zaproxy.org/docs/docker/api-scan/)
