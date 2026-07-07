# Secret Migration Map

## Status

- entries: 132
- pending: 0
- keep: 108
- rotate: 24
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
| NVBES_ACCOUNT_DATABASE_URL | Platform | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-account-database-url` |
| NVBES_ACCOUNT_SERVICE_BASE_URL | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-account-service-base-url` |
| NVBES_ACCOUNT_SERVICE_CLIENT_ID | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-account-service-client-id` |
| NVBES_ACCOUNT_SERVICE_CLIENT_SECRET | Platform | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-account-service-client-secret` |
| NVBES_ACCOUNT_WORKER_METRICS_BIND_ADDR | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-account-worker-metrics-bind-addr` |
| NVBES_ADDITIONAL_CORS_ORIGINS | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-additional-cors-origins` |
| NVBES_ANALYTICS_ID_SALT | Observability | keep | Observability lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-analytics-id-salt` |
| NVBES_API_BASE_URL | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-api-base-url` |
| NVBES_API_PORT | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-api-port` |
| NVBES_APP_NAME | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-app-name` |
| NVBES_AUTH_PASSWORD_RESET_TTL_MINUTES | Identity | keep | Identity lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-auth-password-reset-ttl-minutes` |
| NVBES_AUTH_POW_ENABLED | Identity | keep | Identity lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-auth-pow-enabled` |
| NVBES_AUTH_REFRESH_TOKEN_TTL_HOURS | Identity | keep | Identity lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-auth-refresh-token-ttl-hours` |
| NVBES_AUTH_SESSION_TTL_HOURS | Identity | keep | Identity lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-auth-session-ttl-hours` |
| NVBES_AUTH_STEP_UP_TTL_MINUTES | Identity | keep | Identity lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-auth-step-up-ttl-minutes` |
| NVBES_AUTH_VERIFICATION_TTL_HOURS | Identity | keep | Identity lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-auth-verification-ttl-hours` |
| NVBES_BETA_SEED_EMAIL | Email | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-beta-seed-email` |
| NVBES_BETA_SEED_PASSWORD | Platform | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-beta-seed-password` |
| NVBES_BETA_SEED_WORKSPACE | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-beta-seed-workspace` |
| NVBES_BILLING_CANCEL_URL | Billing/Usage | keep | Billing lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-billing-cancel-url` |
| NVBES_BILLING_DATABASE_URL | Billing/Usage | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-billing-database-url` |
| NVBES_BILLING_EXTERNAL_PROVIDER_FALLBACK_ENABLED | Billing/Usage | keep | Billing lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-billing-external-provider-fallback-enabled` |
| NVBES_BILLING_EXTERNAL_PROVIDER_ROUTING_STATUS | Billing/Usage | keep | Billing lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-billing-external-provider-routing-status` |
| NVBES_BILLING_FRAUD_BLOCK_THRESHOLD | Billing/Usage | keep | Billing lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-billing-fraud-block-threshold` |
| NVBES_BILLING_FRAUD_ENFORCEMENT_ENABLED | Billing/Usage | keep | Billing lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-billing-fraud-enforcement-enabled` |
| NVBES_BILLING_FRAUD_MANUAL_REVIEW_THRESHOLD | Billing/Usage | keep | Billing lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-billing-fraud-manual-review-threshold` |
| NVBES_BILLING_FRAUD_POLICY_OVERRIDES_JSON | Billing/Usage | keep | Billing lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-billing-fraud-policy-overrides-json` |
| NVBES_BILLING_FRAUD_STEP_UP_THRESHOLD | Billing/Usage | keep | Billing lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-billing-fraud-step-up-threshold` |
| NVBES_BILLING_GRPC_ENDPOINT | Billing/Usage | keep | Billing lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-billing-grpc-endpoint` |
| NVBES_BILLING_GRPC_PORT | Billing/Usage | keep | Billing lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-billing-grpc-port` |
| NVBES_BILLING_MOLLIE_ROUTING_STATUS | Billing/Usage | keep | Billing lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-billing-mollie-routing-status` |
| NVBES_BILLING_PORTAL_RETURN_URL | Billing/Usage | keep | Billing lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-billing-portal-return-url` |
| NVBES_BILLING_SERVICE_BASE_URL | Billing/Usage | keep | Billing lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-billing-service-base-url` |
| NVBES_BILLING_SERVICE_PORT | Billing/Usage | keep | Billing lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-billing-service-port` |
| NVBES_BILLING_SUCCESS_URL | Billing/Usage | keep | Billing lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-billing-success-url` |
| NVBES_BILLING_WORKER_METRICS_BIND_ADDR | Billing/Usage | keep | Billing lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-billing-worker-metrics-bind-addr` |
| NVBES_CLOUD_SERVICE_BASE_URL | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-cloud-service-base-url` |
| NVBES_CLOUD_WORKER_METRICS_BIND_ADDR | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-cloud-worker-metrics-bind-addr` |
| NVBES_DATABASE_MAX_CONNECTIONS | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-database-max-connections` |
| NVBES_DATABASE_URL | Platform | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-database-url` |
| NVBES_EMAIL_FROM_EMAIL | Email | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-email-from-email` |
| NVBES_EMAIL_FROM_NAME | Email | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-email-from-name` |
| NVBES_EMAIL_PROVIDER | Email | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-email-provider` |
| NVBES_ENV | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-env` |
| NVBES_GATEWAY_CLOUD_PORT | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-gateway-cloud-port` |
| NVBES_JWT_SECRET | Identity | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-jwt-secret` |
| NVBES_LOG_PII_MASKING | Observability | keep | Observability lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-log-pii-masking` |
| NVBES_LOYALSOLDIER_GEOIP_ENABLED | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-loyalsoldier-geoip-enabled` |
| NVBES_LOYALSOLDIER_GEOIP_LICENSE_ACCEPTED | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-loyalsoldier-geoip-license-accepted` |
| NVBES_MAXMIND_ACCOUNT_ID | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-maxmind-account-id` |
| NVBES_MAXMIND_GEOLITE_DATABASE_ENABLED | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-maxmind-geolite-database-enabled` |
| NVBES_MAXMIND_GEOLITE_EULA_ACCEPTED | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-maxmind-geolite-eula-accepted` |
| NVBES_MAXMIND_GEOLITE_WEB_ENABLED | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-maxmind-geolite-web-enabled` |
| NVBES_MAXMIND_LICENSE_KEY | Platform | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-maxmind-license-key` |
| NVBES_MAXMIND_WEB_CACHE_TTL_HOURS | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-maxmind-web-cache-ttl-hours` |
| NVBES_MAXMIND_WEB_TIMEOUT_SECS | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-maxmind-web-timeout-secs` |
| NVBES_MOLLIE_API_BASE_URL | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-mollie-api-base-url` |
| NVBES_MOLLIE_API_KEY | Platform | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-mollie-api-key` |
| NVBES_MOLLIE_ENABLED | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-mollie-enabled` |
| NVBES_OBSERVABILITY_INTERNAL_TOKEN | Platform | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-observability-internal-token` |
| NVBES_OTLP_AUTHORIZATION_HEADER | Identity | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-otlp-authorization-header` |
| NVBES_OTLP_ENDPOINT | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-otlp-endpoint` |
| NVBES_OTP_PROVIDER | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-otp-provider` |
| NVBES_POSTHOG_ENABLED | Observability | keep | Observability lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-posthog-enabled` |
| NVBES_POSTHOG_HOST | Observability | keep | Observability lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-posthog-host` |
| NVBES_POSTHOG_PROJECT_TOKEN | Observability | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-posthog-project-token` |
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
| NVBES_STAGING_ACCOUNT_SERVICE_BASE_URL | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-staging-account-service-base-url` |
| NVBES_STAGING_API_BASE_URL | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-staging-api-base-url` |
| NVBES_STAGING_BILLING_DATABASE_URL | Billing/Usage | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-staging-billing-database-url` |
| NVBES_STAGING_CLOUD_SERVICE_BASE_URL | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-staging-cloud-service-base-url` |
| NVBES_STAGING_DATABASE_URL | Platform | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-staging-database-url` |
| NVBES_STAGING_WEB_BASE_URL | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-staging-web-base-url` |
| NVBES_STRIPE_API_BASE_URL | Billing/Usage | keep | Billing lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-stripe-api-base-url` |
| NVBES_STRIPE_SECRET_KEY | Billing/Usage | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-stripe-secret-key` |
| NVBES_STRIPE_WEBHOOK_SECRET | Billing/Usage | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-stripe-webhook-secret` |
| NVBES_TLS_CERT_PATH | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-tls-cert-path` |
| NVBES_TLS_ENABLED | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-tls-enabled` |
| NVBES_TLS_KEY_PATH | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-tls-key-path` |
| NVBES_TWILIO_API_BASE_URL | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-twilio-api-base-url` |
| NVBES_WEB_BASE_URL | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-web-base-url` |
| QUARANTINE_RETENTION_DAYS | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.quarantine-retention-days` |
| SCAN_ENABLED | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.scan-enabled` |
| SCAN_ENGINE | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.scan-engine` |
| SCAN_FAIL_OPEN | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.scan-fail-open` |
| SCAN_TIMEOUT_SECS | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.scan-timeout-secs` |
| SENTRY_AUTH_TOKEN | Identity | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.sentry-auth-token` |
| SENTRY_DSN | Observability | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.sentry-dsn` |
| SENTRY_ORG | Observability | keep | Observability lead | not-required | `deploy/oss/helm/nvbes#config.sentry-org` |
| SENTRY_PROJECT | Observability | keep | Observability lead | not-required | `deploy/oss/helm/nvbes#config.sentry-project` |
| SENTRY_RELEASE | Observability | keep | Observability lead | not-required | `deploy/oss/helm/nvbes#config.sentry-release` |
| SENTRY_TRACES_SAMPLE_RATE | Observability | keep | Observability lead | not-required | `deploy/oss/helm/nvbes#config.sentry-traces-sample-rate` |
| SENTRY_URL | Observability | keep | Observability lead | not-required | `deploy/oss/helm/nvbes#config.sentry-url` |
| STORAGE_BUCKET | Drive | keep | Drive lead | not-required | `deploy/oss/helm/nvbes#config.storage-bucket` |
| STORAGE_ENABLED | Drive | keep | Drive lead | not-required | `deploy/oss/helm/nvbes#config.storage-enabled` |
| STORAGE_ENDPOINT | Drive | keep | Drive lead | not-required | `deploy/oss/helm/nvbes#config.storage-endpoint` |
| STORAGE_REGION | Drive | keep | Drive lead | not-required | `deploy/oss/helm/nvbes#config.storage-region` |
| VITE_ACCOUNT_CLIENT_ID | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#public-env.vite-account-client-id` |
| VITE_ACCOUNT_SERVICE_BASE_URL | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#public-env.vite-account-service-base-url` |
| VITE_ACCOUNT_WEB_BASE_URL | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#public-env.vite-account-web-base-url` |
| VITE_ANALYTICS_ID_SALT | Observability | keep | Observability lead | not-required | `deploy/oss/helm/nvbes#public-env.vite-analytics-id-salt` |
| VITE_BILLING_SERVICE_BASE_URL | Billing/Usage | keep | Billing lead | not-required | `deploy/oss/helm/nvbes#public-env.vite-billing-service-base-url` |
| VITE_CLOUD_SERVICE_BASE_URL | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#public-env.vite-cloud-service-base-url` |
| VITE_CONSOLE_ACCOUNT_CLIENT_ID | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#public-env.vite-console-account-client-id` |
| VITE_CONSOLE_WEB_BASE_URL | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#public-env.vite-console-web-base-url` |
| VITE_FARO_API_KEY | Platform | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#public-env.vite-faro-api-key` |
| VITE_FARO_RELEASE | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#public-env.vite-faro-release` |
| VITE_FARO_SESSION_SAMPLE_RATE | Identity | keep | Identity lead | not-required | `deploy/oss/helm/nvbes#public-env.vite-faro-session-sample-rate` |
| VITE_FARO_TRACING_ORIGINS | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#public-env.vite-faro-tracing-origins` |
| VITE_FARO_URL | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#public-env.vite-faro-url` |
| VITE_GRAFANA_FARO_URL | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#public-env.vite-grafana-faro-url` |
| VITE_POSTHOG_HOST | Observability | keep | Observability lead | not-required | `deploy/oss/helm/nvbes#public-env.vite-posthog-host` |
| VITE_POSTHOG_KEY | Observability | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#public-env.vite-posthog-key` |
| VITE_REACT_QUERY_DEVTOOLS_ENABLED | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#public-env.vite-react-query-devtools-enabled` |
| VITE_SENTRY_DSN | Observability | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#public-env.vite-sentry-dsn` |
| VITE_SENTRY_RELEASE | Observability | keep | Observability lead | not-required | `deploy/oss/helm/nvbes#public-env.vite-sentry-release` |
| VITE_SENTRY_TRACES_SAMPLE_RATE | Observability | keep | Observability lead | not-required | `deploy/oss/helm/nvbes#public-env.vite-sentry-traces-sample-rate` |
| VITE_TANSTACK_ROUTER_DEVTOOLS_ENABLED | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#public-env.vite-tanstack-router-devtools-enabled` |

## Regeneration

```bash
pnpm check:migration-secret-map
node tools/migration/secret-map.mjs --write
```
