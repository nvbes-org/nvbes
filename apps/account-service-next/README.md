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

The service runs its Account-only SQL migrations at startup. Identity
registration events initialize the profile and settings through a versioned,
fingerprinted inbox. Profile changes are projected back to Identity for OIDC
claims through a versioned outbox; no inter-service foreign key or synchronous
Identity data lookup is used.

Account closure is a durable saga. `POST /api/v1/closure` returns `202 Accepted`
with the stable saga identifier and atomically publishes
`account.closure.requested.v1`. Account Worker executes four persisted,
ordered checkpoints: Cloud, Billing, Identity, then Account. Every remote
participant has a fingerprinted inbox, so a crash after a remote success can
replay the same event safely. The Account checkpoint deletes the avatar, purges
Account-owned user data and completes the saga in the same transaction as the
outbox acknowledgement. `GET /api/v1/closure` exposes the saga and participant
states. Transient failures use bounded exponential retries; terminal failures
remain observable on the participant, saga and dead-lettered event.

Account Worker requires the three participant base URLs and dedicated tokens:

```text
NVBES_CLOUD_SERVICE_BASE_URL
NVBES_CLOUD_INTERNAL_TOKEN
NVBES_BILLING_SERVICE_BASE_URL
NVBES_BILLING_INTERNAL_TOKEN
NVBES_IDENTITY_SERVICE_BASE_URL
NVBES_IDENTITY_INTERNAL_TOKEN
```

Generate the contract after the crate replaces the legacy package in the Cargo
workspace:

```bash
pnpm nx run account-service:export-openapi
```
