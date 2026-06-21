# Secret Migration Map

## Status

- entries: 77
- pending: 0
- keep: 64
- rotate: 13
- remove: 0
- replace: 0

## Rules

- Every secret or config key must have a target, owner, rotation decision and verification checks.
- Source rows, domains and summary counters must match the inventory-derived map.
- Generation provenance must identify source, write command and strict cutover command.

## Secrets

| Key | Domain | Decision | Owner | Rotation | Target |
|---|---|---:|---|---|---|
| CLAMAV_HOST | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.clamav-host` |
| CLAMAV_PORT | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.clamav-port` |
| NVBES_ADDITIONAL_CORS_ORIGINS | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-additional-cors-origins` |
| NVBES_ANALYTICS_ID_SALT | Observability | keep | Observability lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-analytics-id-salt` |
| NVBES_API_BASE_URL | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-api-base-url` |
| NVBES_API_PORT | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-api-port` |
| NVBES_APP_NAME | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-app-name` |
| NVBES_AUTH_PASSWORD_RESET_TTL_MINUTES | Identity | keep | Identity lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-auth-password-reset-ttl-minutes` |
| NVBES_AUTH_REFRESH_TOKEN_TTL_HOURS | Identity | keep | Identity lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-auth-refresh-token-ttl-hours` |
| NVBES_AUTH_SESSION_TTL_HOURS | Identity | keep | Identity lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-auth-session-ttl-hours` |
| NVBES_AUTH_STEP_UP_TTL_MINUTES | Identity | keep | Identity lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-auth-step-up-ttl-minutes` |
| NVBES_AUTH_VERIFICATION_TTL_HOURS | Identity | keep | Identity lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-auth-verification-ttl-hours` |
| NVBES_BETA_SEED_EMAIL | Email | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-beta-seed-email` |
| NVBES_BETA_SEED_PASSWORD | Platform | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-beta-seed-password` |
| NVBES_BETA_SEED_WORKSPACE | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-beta-seed-workspace` |
| NVBES_BILLING_CANCEL_URL | Billing/Usage | keep | Billing lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-billing-cancel-url` |
| NVBES_BILLING_PORTAL_RETURN_URL | Billing/Usage | keep | Billing lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-billing-portal-return-url` |
| NVBES_BILLING_SUCCESS_URL | Billing/Usage | keep | Billing lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-billing-success-url` |
| NVBES_DATABASE_MAX_CONNECTIONS | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-database-max-connections` |
| NVBES_DATABASE_URL | Platform | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-database-url` |
| NVBES_DRIVE_API_BASE_URL | Drive | keep | Drive lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-drive-api-base-url` |
| NVBES_DRIVE_WORKER_METRICS_BIND_ADDR | Drive | keep | Drive lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-drive-worker-metrics-bind-addr` |
| NVBES_EMAIL_FROM_EMAIL | Email | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-email-from-email` |
| NVBES_EMAIL_FROM_NAME | Email | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-email-from-name` |
| NVBES_EMAIL_PROVIDER | Email | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-email-provider` |
| NVBES_ENV | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-env` |
| NVBES_IDENTITY_API_BASE_URL | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-identity-api-base-url` |
| NVBES_IDENTITY_BASE_URL | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-identity-base-url` |
| NVBES_IDENTITY_CLIENT_ID | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-identity-client-id` |
| NVBES_IDENTITY_CLIENT_SECRET | Platform | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-identity-client-secret` |
| NVBES_IDENTITY_WORKER_METRICS_BIND_ADDR | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-identity-worker-metrics-bind-addr` |
| NVBES_JWT_SECRET | Identity | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-jwt-secret` |
| NVBES_LOG_PII_MASKING | Observability | keep | Observability lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-log-pii-masking` |
| NVBES_OBSERVABILITY_INTERNAL_TOKEN | Platform | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-observability-internal-token` |
| NVBES_OTLP_AUTHORIZATION_HEADER | Identity | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-otlp-authorization-header` |
| NVBES_OTLP_ENDPOINT | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-otlp-endpoint` |
| NVBES_PRODUCT_ANALYTICS_ENABLED | Observability | keep | Observability lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-product-analytics-enabled` |
| NVBES_PRODUCT_ANALYTICS_TOKEN | Observability | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-product-analytics-token` |
| NVBES_PROFILING_BASIC_AUTH_PASSWORD | Identity | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-profiling-basic-auth-password` |
| NVBES_PROFILING_BASIC_AUTH_USER | Identity | keep | Identity lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-profiling-basic-auth-user` |
| NVBES_PROFILING_ENABLED | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-profiling-enabled` |
| NVBES_PROFILING_ENDPOINT | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-profiling-endpoint` |
| NVBES_PROFILING_SAMPLE_RATE_HZ | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-profiling-sample-rate-hz` |
| NVBES_REDIS_MAX_CONNECTIONS | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-redis-max-connections` |
| NVBES_REDIS_URL | Platform | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-redis-url` |
| NVBES_REQUEST_E2EE_ENABLED | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-request-e2ee-enabled` |
| NVBES_REQUEST_E2EE_KEY_ID | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-request-e2ee-key-id` |
| NVBES_REQUEST_E2EE_REQUIRED | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-request-e2ee-required` |
| NVBES_REQUEST_E2EE_SECRET | Platform | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-request-e2ee-secret` |
| NVBES_STAGING_API_BASE_URL | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-staging-api-base-url` |
| NVBES_STAGING_DATABASE_URL | Platform | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-staging-database-url` |
| NVBES_STAGING_DRIVE_API_BASE_URL | Drive | keep | Drive lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-staging-drive-api-base-url` |
| NVBES_STAGING_IDENTITY_API_BASE_URL | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-staging-identity-api-base-url` |
| NVBES_STAGING_WEB_BASE_URL | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-staging-web-base-url` |
| NVBES_STRIPE_API_BASE_URL | Billing/Usage | keep | Billing lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-stripe-api-base-url` |
| NVBES_STRIPE_SECRET_KEY | Billing/Usage | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-stripe-secret-key` |
| NVBES_STRIPE_WEBHOOK_SECRET | Billing/Usage | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-stripe-webhook-secret` |
| NVBES_TLS_CERT_PATH | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-tls-cert-path` |
| NVBES_TLS_ENABLED | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-tls-enabled` |
| NVBES_TLS_KEY_PATH | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-tls-key-path` |
| NVBES_WEB_BASE_URL | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-web-base-url` |
| QUARANTINE_RETENTION_DAYS | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.quarantine-retention-days` |
| SCAN_ENABLED | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.scan-enabled` |
| SCAN_ENGINE | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.scan-engine` |
| SCAN_FAIL_OPEN | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.scan-fail-open` |
| SCAN_TIMEOUT_SECS | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.scan-timeout-secs` |
| STORAGE_BUCKET | Drive | keep | Drive lead | not-required | `deploy/oss/helm/nvbes#config.storage-bucket` |
| STORAGE_ENABLED | Drive | keep | Drive lead | not-required | `deploy/oss/helm/nvbes#config.storage-enabled` |
| STORAGE_ENDPOINT | Drive | keep | Drive lead | not-required | `deploy/oss/helm/nvbes#config.storage-endpoint` |
| STORAGE_REGION | Drive | keep | Drive lead | not-required | `deploy/oss/helm/nvbes#config.storage-region` |
| VITE_ANALYTICS_ID_SALT | Observability | keep | Observability lead | not-required | `deploy/oss/helm/nvbes#public-env.vite-analytics-id-salt` |
| VITE_DEVELOPER_IDENTITY_CLIENT_ID | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#public-env.vite-developer-identity-client-id` |
| VITE_DEVELOPER_WEB_BASE_URL | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#public-env.vite-developer-web-base-url` |
| VITE_DRIVE_API_BASE_URL | Drive | keep | Drive lead | not-required | `deploy/oss/helm/nvbes#public-env.vite-drive-api-base-url` |
| VITE_IDENTITY_API_BASE_URL | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#public-env.vite-identity-api-base-url` |
| VITE_IDENTITY_CLIENT_ID | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#public-env.vite-identity-client-id` |
| VITE_IDENTITY_WEB_BASE_URL | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#public-env.vite-identity-web-base-url` |

## Regeneration

```bash
pnpm check:migration-secret-map
tools/migration/secret-map.mjs --write
```
