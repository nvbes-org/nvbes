# nvbes Account Service

OAuth 2.0 Resource Server for the Account product. It owns profile, display
preferences, notification preferences, legal consents, Account privacy exports,
avatar metadata and Account closure orchestration.

It does **not** own authentication, browser sessions, OAuth clients, JWT signing,
refresh tokens, email identities, MFA or Identity persistence. Every product API
requires an Identity access token with:

- issuer `NVBES_IDENTITY_SERVICE_BASE_URL`;
- exact audience `nvbes-account-service`;
- the exact scope documented on the operation.

Cookies are ignored. CORS admits only the origin derived from
`NVBES_ACCOUNT_WEB_BASE_URL` and never enables credentialed requests.

## Required configuration

```text
NVBES_ACCOUNT_DATABASE_URL
NVBES_ACCOUNT_SERVICE_PORT
NVBES_ACCOUNT_SERVICE_BASE_URL
NVBES_IDENTITY_SERVICE_BASE_URL
NVBES_ACCOUNT_WEB_BASE_URL
NVBES_ACCOUNT_AVATAR_STORAGE_MODE=s3|mock
```

S3 mode additionally requires:

```text
NVBES_ACCOUNT_AVATAR_STORAGE_BUCKET
NVBES_ACCOUNT_AVATAR_STORAGE_ENDPOINT
NVBES_ACCOUNT_AVATAR_STORAGE_REGION
NVBES_ACCOUNT_AVATAR_STORAGE_ACCESS_KEY
NVBES_ACCOUNT_AVATAR_STORAGE_SECRET_KEY
```

`NVBES_ACCOUNT_AVATAR_STORAGE_PUBLIC_ENDPOINT` is optional. Mock storage is
accepted only when `NVBES_ENVIRONMENT` is `development` or `test`.

## Runtime

The service runs its Account-only SQL migrations at startup. Profiles and
settings are initialized from the verified Identity subject on first use; no
inter-service foreign key or synchronous Identity data lookup is performed.

Account closure is intentionally a saga: `POST /api/v1/closure` atomically
creates an `account_closure_sagas` row and an `account_outbox_events` event.
It does not pretend that Identity has already deleted the principal.

Generate the contract after the crate replaces the legacy package in the Cargo
workspace:

```bash
pnpm nx run account-service:export-openapi
```

