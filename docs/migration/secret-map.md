# Secret Migration Map

## Status

- entries: 248
- pending: 0
- keep: 185
- rotate: 63
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
| GRAFANA_CLOUD_STACK_ID | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.grafana-cloud-stack-id` |
| GRAFANA_FARO_APP_ID_ACCOUNT_WEB | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.grafana-faro-app-id-account-web` |
| GRAFANA_FARO_APP_ID_BACKOFFICE_WEB | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.grafana-faro-app-id-backoffice-web` |
| GRAFANA_FARO_APP_ID_CLOUD_WEB | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.grafana-faro-app-id-cloud-web` |
| GRAFANA_FARO_APP_ID_CONSOLE_WEB | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.grafana-faro-app-id-console-web` |
| GRAFANA_FARO_APP_ID_ENTERPRISE_WEB | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.grafana-faro-app-id-enterprise-web` |
| GRAFANA_FARO_SOURCEMAP_API_KEY | Platform | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.grafana-faro-sourcemap-api-key` |
| GRAFANA_FARO_SOURCEMAP_ENDPOINT | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.grafana-faro-sourcemap-endpoint` |
| GRAFANA_FARO_SOURCEMAP_UPLOAD_ENABLED | Drive | keep | Drive lead | not-required | `deploy/oss/helm/nvbes#config.grafana-faro-sourcemap-upload-enabled` |
| NVBES_ACCOUNT_AVATAR_STORAGE_ACCESS_KEY | Drive | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-account-avatar-storage-access-key` |
| NVBES_ACCOUNT_AVATAR_STORAGE_BUCKET | Drive | keep | Drive lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-account-avatar-storage-bucket` |
| NVBES_ACCOUNT_AVATAR_STORAGE_ENDPOINT | Drive | keep | Drive lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-account-avatar-storage-endpoint` |
| NVBES_ACCOUNT_AVATAR_STORAGE_MODE | Drive | keep | Drive lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-account-avatar-storage-mode` |
| NVBES_ACCOUNT_AVATAR_STORAGE_PUBLIC_ENDPOINT | Drive | keep | Drive lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-account-avatar-storage-public-endpoint` |
| NVBES_ACCOUNT_AVATAR_STORAGE_REGION | Drive | keep | Drive lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-account-avatar-storage-region` |
| NVBES_ACCOUNT_AVATAR_STORAGE_SECRET_KEY | Drive | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-account-avatar-storage-secret-key` |
| NVBES_ACCOUNT_DATABASE_URL | Platform | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-account-database-url` |
| NVBES_ACCOUNT_EXPORT_FRAGMENT_MAX_BYTES | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-account-export-fragment-max-bytes` |
| NVBES_ACCOUNT_MIGRATION_DATABASE_URL | Platform | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-account-migration-database-url` |
| NVBES_ACCOUNT_PROVISIONING_TOKEN | Platform | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-account-provisioning-token` |
| NVBES_ACCOUNT_SERVICE_BASE_URL | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-account-service-base-url` |
| NVBES_ACCOUNT_SERVICE_PORT | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-account-service-port` |
| NVBES_ACCOUNT_WEB_BASE_URL | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-account-web-base-url` |
| NVBES_ACCOUNT_WEB_OAUTH_REDIRECT_URIS | Identity | keep | Identity lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-account-web-oauth-redirect-uris` |
| NVBES_ACCOUNT_WORKER_METRICS_BIND_ADDR | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-account-worker-metrics-bind-addr` |
| NVBES_ADDITIONAL_CORS_ORIGINS | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-additional-cors-origins` |
| NVBES_ANALYTICS_ID_SALT | Observability | keep | Observability lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-analytics-id-salt` |
| NVBES_API_BASE_URL | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-api-base-url` |
| NVBES_API_MAX_CONCURRENT_REQUESTS | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-api-max-concurrent-requests` |
| NVBES_API_MAX_CONNECTIONS_PER_IP | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-api-max-connections-per-ip` |
| NVBES_API_PORT | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-api-port` |
| NVBES_APP_NAME | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-app-name` |
| NVBES_AUTH_FACTOR_ENCRYPTION_KEY | Identity | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-auth-factor-encryption-key` |
| NVBES_AUTH_FACTOR_ENCRYPTION_KEY_VERSION | Identity | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-auth-factor-encryption-key-version` |
| NVBES_AUTH_PASSWORD_RESET_TTL_MINUTES | Identity | keep | Identity lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-auth-password-reset-ttl-minutes` |
| NVBES_AUTH_POW_ENABLED | Identity | keep | Identity lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-auth-pow-enabled` |
| NVBES_AUTH_REFRESH_TOKEN_TTL_HOURS | Identity | keep | Identity lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-auth-refresh-token-ttl-hours` |
| NVBES_AUTH_SESSION_IDLE_TTL_MINUTES | Identity | keep | Identity lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-auth-session-idle-ttl-minutes` |
| NVBES_AUTH_SESSION_TTL_HOURS | Identity | keep | Identity lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-auth-session-ttl-hours` |
| NVBES_AUTH_STEP_UP_TTL_MINUTES | Identity | keep | Identity lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-auth-step-up-ttl-minutes` |
| NVBES_AUTH_VERIFICATION_TTL_HOURS | Identity | keep | Identity lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-auth-verification-ttl-hours` |
| NVBES_BACKOFFICE_IDENTITY_CLIENT_ID | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-backoffice-identity-client-id` |
| NVBES_BACKOFFICE_IDENTITY_CLIENT_SECRET | Platform | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-backoffice-identity-client-secret` |
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
| NVBES_BILLING_IDENTITY_CLIENT_ID | Billing/Usage | keep | Billing lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-billing-identity-client-id` |
| NVBES_BILLING_IDENTITY_CLIENT_SECRET | Billing/Usage | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-billing-identity-client-secret` |
| NVBES_BILLING_INTERNAL_TOKEN | Billing/Usage | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-billing-internal-token` |
| NVBES_BILLING_MOLLIE_ROUTING_STATUS | Billing/Usage | keep | Billing lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-billing-mollie-routing-status` |
| NVBES_BILLING_PORTAL_RETURN_URL | Billing/Usage | keep | Billing lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-billing-portal-return-url` |
| NVBES_BILLING_SERVICE_BASE_URL | Billing/Usage | keep | Billing lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-billing-service-base-url` |
| NVBES_BILLING_SERVICE_PORT | Billing/Usage | keep | Billing lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-billing-service-port` |
| NVBES_BILLING_SUCCESS_URL | Billing/Usage | keep | Billing lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-billing-success-url` |
| NVBES_BILLING_WORKER_METRICS_BIND_ADDR | Billing/Usage | keep | Billing lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-billing-worker-metrics-bind-addr` |
| NVBES_CLOUD_DATABASE_URL | Platform | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-cloud-database-url` |
| NVBES_CLOUD_IDENTITY_CLIENT_ID | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-cloud-identity-client-id` |
| NVBES_CLOUD_IDENTITY_CLIENT_SECRET | Platform | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-cloud-identity-client-secret` |
| NVBES_CLOUD_INTERNAL_TOKEN | Platform | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-cloud-internal-token` |
| NVBES_CLOUD_RUNTIME_DATABASE_URL | Platform | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-cloud-runtime-database-url` |
| NVBES_CLOUD_SERVICE_BASE_URL | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-cloud-service-base-url` |
| NVBES_CLOUD_SYSTEM_DATABASE_URL | Platform | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-cloud-system-database-url` |
| NVBES_CLOUD_WORKER_METRICS_BIND_ADDR | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-cloud-worker-metrics-bind-addr` |
| NVBES_DATABASE_MAX_CONNECTIONS | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-database-max-connections` |
| NVBES_DATABASE_URL | Platform | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-database-url` |
| NVBES_DEVELOPER_ALLOWED_ORIGINS | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-developer-allowed-origins` |
| NVBES_DEVELOPER_DATABASE_URL | Platform | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-developer-database-url` |
| NVBES_DEVELOPER_GRPC_PORT | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-developer-grpc-port` |
| NVBES_DEVELOPER_HTTP_PORT | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-developer-http-port` |
| NVBES_DEVELOPER_IDENTITY_CLIENT_ID | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-developer-identity-client-id` |
| NVBES_DEVELOPER_IDENTITY_CLIENT_SECRET | Platform | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-developer-identity-client-secret` |
| NVBES_EMAIL_DATA_ENCRYPTION_KEY | Email | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-email-data-encryption-key` |
| NVBES_EMAIL_DATABASE_URL | Email | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-email-database-url` |
| NVBES_EMAIL_FROM_EMAIL | Email | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-email-from-email` |
| NVBES_EMAIL_FROM_NAME | Email | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-email-from-name` |
| NVBES_EMAIL_GRPC_AUTH_TOKEN | Identity | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-email-grpc-auth-token` |
| NVBES_EMAIL_GRPC_BIND_ADDR | Email | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-email-grpc-bind-addr` |
| NVBES_EMAIL_GRPC_ENDPOINT | Email | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-email-grpc-endpoint` |
| NVBES_EMAIL_HTTP_BIND_ADDR | Email | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-email-http-bind-addr` |
| NVBES_EMAIL_LEDGER_RETENTION_DAYS | Email | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-email-ledger-retention-days` |
| NVBES_EMAIL_MESSAGE_ID_DOMAIN | Email | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-email-message-id-domain` |
| NVBES_EMAIL_PAYLOAD_RETENTION_DAYS | Email | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-email-payload-retention-days` |
| NVBES_EMAIL_PRODUCER_TOKENS | Email | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-email-producer-tokens` |
| NVBES_EMAIL_PROVIDER | Email | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-email-provider` |
| NVBES_EMAIL_RECIPIENT_HMAC_KEY | Email | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-email-recipient-hmac-key` |
| NVBES_ENTERPRISE_DATABASE_URL | Platform | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-enterprise-database-url` |
| NVBES_ENTERPRISE_GRPC_AUTH_TOKEN | Identity | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-enterprise-grpc-auth-token` |
| NVBES_ENTERPRISE_GRPC_ENDPOINT | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-enterprise-grpc-endpoint` |
| NVBES_ENV | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-env` |
| NVBES_FAPI_CONFORMANCE_EVIDENCE_SHA256 | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-fapi-conformance-evidence-sha256` |
| NVBES_FAPI_HIGH_ASSURANCE_ENABLED | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-fapi-high-assurance-enabled` |
| NVBES_GATEWAY_CLOUD_PORT | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-gateway-cloud-port` |
| NVBES_GATEWAY_IDENTITY_CLIENT_ID | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-gateway-identity-client-id` |
| NVBES_GATEWAY_IDENTITY_CLIENT_SECRET | Platform | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-gateway-identity-client-secret` |
| NVBES_HTTP_REQUEST_TIMEOUT_SECS | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-http-request-timeout-secs` |
| NVBES_IDENTITY_DATABASE_URL | Platform | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-identity-database-url` |
| NVBES_IDENTITY_GRPC_ENDPOINT | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-identity-grpc-endpoint` |
| NVBES_IDENTITY_GRPC_PORT | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-identity-grpc-port` |
| NVBES_IDENTITY_INTERNAL_TOKEN | Platform | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-identity-internal-token` |
| NVBES_IDENTITY_SERVICE_BASE_URL | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-identity-service-base-url` |
| NVBES_IDENTITY_SERVICE_PORT | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-identity-service-port` |
| NVBES_IDENTITY_TEST_DATABASE_URL | Platform | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-identity-test-database-url` |
| NVBES_IDENTITY_WEB_BASE_URL | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-identity-web-base-url` |
| NVBES_JWT_SECRET | Identity | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-jwt-secret` |
| NVBES_KMS_API_BASE_URL | Platform | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-kms-api-base-url` |
| NVBES_KMS_AUTH_TOKEN | Identity | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-kms-auth-token` |
| NVBES_KMS_ENABLED | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-kms-enabled` |
| NVBES_KMS_KEY_ID | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-kms-key-id` |
| NVBES_KMS_REGION | Platform | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-kms-region` |
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
| NVBES_MTLS_CLIENT_CERT_PATH | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-mtls-client-cert-path` |
| NVBES_MTLS_CLIENT_KEY_PATH | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-mtls-client-key-path` |
| NVBES_MTLS_ENABLED | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-mtls-enabled` |
| NVBES_MTLS_PORT | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-mtls-port` |
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
| NVBES_REDIS_PASSWORD | Platform | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-redis-password` |
| NVBES_REDIS_URL | Platform | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-redis-url` |
| NVBES_RELEASE | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-release` |
| NVBES_REQUEST_E2EE_ENABLED | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-request-e2ee-enabled` |
| NVBES_REQUEST_E2EE_KEY_ID | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-request-e2ee-key-id` |
| NVBES_REQUEST_E2EE_REQUIRED | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-request-e2ee-required` |
| NVBES_REQUEST_E2EE_SECRET | Platform | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-request-e2ee-secret` |
| NVBES_REQUIRE_FARO_ACCOUNT_WEB | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-require-faro-account-web` |
| NVBES_SECRET_MANAGER_API_BASE_URL | Platform | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-secret-manager-api-base-url` |
| NVBES_SECRET_MANAGER_AUTH_TOKEN | Identity | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-secret-manager-auth-token` |
| NVBES_SECRET_MANAGER_ENABLED | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-secret-manager-enabled` |
| NVBES_SECRET_MANAGER_REGION | Platform | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-secret-manager-region` |
| NVBES_SECRET_MANAGER_SECRET_ID | Platform | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-secret-manager-secret-id` |
| NVBES_SMTP_HOST | Email | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-smtp-host` |
| NVBES_SMTP_PASSWORD | Email | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.nvbes-smtp-password` |
| NVBES_SMTP_PORT | Email | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-smtp-port` |
| NVBES_SMTP_STARTTLS | Email | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-smtp-starttls` |
| NVBES_SMTP_USERNAME | Email | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-smtp-username` |
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
| NVBES_TLS_CLIENT_CA_PATH | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-tls-client-ca-path` |
| NVBES_TLS_ENABLED | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-tls-enabled` |
| NVBES_TLS_KEY_PATH | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-tls-key-path` |
| NVBES_TRUSTED_PROXY_CIDRS | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-trusted-proxy-cidrs` |
| NVBES_TWILIO_API_BASE_URL | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-twilio-api-base-url` |
| NVBES_WEB_BASE_URL | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.nvbes-web-base-url` |
| POSTHOG_CLI_API_KEY | Observability | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.posthog-cli-api-key` |
| POSTHOG_CLI_HOST | Observability | keep | Observability lead | not-required | `deploy/oss/helm/nvbes#config.posthog-cli-host` |
| POSTHOG_CLI_PROJECT_ID | Observability | keep | Observability lead | not-required | `deploy/oss/helm/nvbes#config.posthog-cli-project-id` |
| POSTHOG_SOURCEMAP_UPLOAD_ENABLED | Drive | keep | Drive lead | not-required | `deploy/oss/helm/nvbes#config.posthog-sourcemap-upload-enabled` |
| QUARANTINE_RETENTION_DAYS | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.quarantine-retention-days` |
| SCAN_ENABLED | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.scan-enabled` |
| SCAN_ENGINE | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.scan-engine` |
| SCAN_FAIL_OPEN | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.scan-fail-open` |
| SCAN_TIMEOUT_SECS | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#config.scan-timeout-secs` |
| SENTRY_AUTH_TOKEN | Identity | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.sentry-auth-token` |
| SENTRY_DSN | Observability | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.sentry-dsn` |
| SENTRY_ORG | Observability | keep | Observability lead | not-required | `deploy/oss/helm/nvbes#config.sentry-org` |
| SENTRY_PROJECT | Observability | keep | Observability lead | not-required | `deploy/oss/helm/nvbes#config.sentry-project` |
| SENTRY_PROJECT_ACCOUNT_WEB | Observability | keep | Observability lead | not-required | `deploy/oss/helm/nvbes#config.sentry-project-account-web` |
| SENTRY_PROJECT_BACKOFFICE_WEB | Observability | keep | Observability lead | not-required | `deploy/oss/helm/nvbes#config.sentry-project-backoffice-web` |
| SENTRY_PROJECT_CLOUD_WEB | Observability | keep | Observability lead | not-required | `deploy/oss/helm/nvbes#config.sentry-project-cloud-web` |
| SENTRY_PROJECT_CONSOLE_WEB | Observability | keep | Observability lead | not-required | `deploy/oss/helm/nvbes#config.sentry-project-console-web` |
| SENTRY_PROJECT_ENTERPRISE_WEB | Observability | keep | Observability lead | not-required | `deploy/oss/helm/nvbes#config.sentry-project-enterprise-web` |
| SENTRY_RELEASE | Observability | keep | Observability lead | not-required | `deploy/oss/helm/nvbes#config.sentry-release` |
| SENTRY_SOURCEMAP_UPLOAD_ENABLED | Drive | keep | Drive lead | not-required | `deploy/oss/helm/nvbes#config.sentry-sourcemap-upload-enabled` |
| SENTRY_TRACES_SAMPLE_RATE | Observability | keep | Observability lead | not-required | `deploy/oss/helm/nvbes#config.sentry-traces-sample-rate` |
| SENTRY_URL | Observability | keep | Observability lead | not-required | `deploy/oss/helm/nvbes#config.sentry-url` |
| STORAGE_ACCESS_KEY | Drive | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.storage-access-key` |
| STORAGE_BUCKET | Drive | keep | Drive lead | not-required | `deploy/oss/helm/nvbes#config.storage-bucket` |
| STORAGE_ENABLED | Drive | keep | Drive lead | not-required | `deploy/oss/helm/nvbes#config.storage-enabled` |
| STORAGE_ENDPOINT | Drive | keep | Drive lead | not-required | `deploy/oss/helm/nvbes#config.storage-endpoint` |
| STORAGE_PUBLIC_ENDPOINT | Drive | keep | Drive lead | not-required | `deploy/oss/helm/nvbes#config.storage-public-endpoint` |
| STORAGE_REGION | Drive | keep | Drive lead | not-required | `deploy/oss/helm/nvbes#config.storage-region` |
| STORAGE_SECRET_KEY | Drive | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#external-secret.storage-secret-key` |
| VITE_ACCOUNT_CLIENT_ID | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#public-env.vite-account-client-id` |
| VITE_ACCOUNT_OAUTH_CLIENT_ID | Identity | keep | Identity lead | not-required | `deploy/oss/helm/nvbes#public-env.vite-account-oauth-client-id` |
| VITE_ACCOUNT_OAUTH_REDIRECT_URI | Identity | keep | Identity lead | not-required | `deploy/oss/helm/nvbes#public-env.vite-account-oauth-redirect-uri` |
| VITE_ACCOUNT_SERVICE_BASE_URL | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#public-env.vite-account-service-base-url` |
| VITE_ACCOUNT_WEB_BASE_URL | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#public-env.vite-account-web-base-url` |
| VITE_ANALYTICS_ID_SALT | Observability | keep | Observability lead | not-required | `deploy/oss/helm/nvbes#public-env.vite-analytics-id-salt` |
| VITE_BILLING_SERVICE_BASE_URL | Billing/Usage | keep | Billing lead | not-required | `deploy/oss/helm/nvbes#public-env.vite-billing-service-base-url` |
| VITE_CLOUD_SERVICE_BASE_URL | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#public-env.vite-cloud-service-base-url` |
| VITE_CONSOLE_ACCOUNT_CLIENT_ID | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#public-env.vite-console-account-client-id` |
| VITE_CONSOLE_WEB_BASE_URL | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#public-env.vite-console-web-base-url` |
| VITE_DEVELOPER_SERVICE_BASE_URL | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#public-env.vite-developer-service-base-url` |
| VITE_FARO_API_KEY | Platform | rotate | Security lead | required-before-cutover | `deploy/oss/helm/nvbes#public-env.vite-faro-api-key` |
| VITE_FARO_ENVIRONMENT | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#public-env.vite-faro-environment` |
| VITE_FARO_RELEASE | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#public-env.vite-faro-release` |
| VITE_FARO_SESSION_SAMPLE_RATE | Identity | keep | Identity lead | not-required | `deploy/oss/helm/nvbes#public-env.vite-faro-session-sample-rate` |
| VITE_FARO_TRACING_ORIGINS | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#public-env.vite-faro-tracing-origins` |
| VITE_FARO_URL | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#public-env.vite-faro-url` |
| VITE_FARO_URL_ACCOUNT_WEB | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#public-env.vite-faro-url-account-web` |
| VITE_GRAFANA_FARO_URL | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#public-env.vite-grafana-faro-url` |
| VITE_IDENTITY_SERVICE_BASE_URL | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#public-env.vite-identity-service-base-url` |
| VITE_IDENTITY_WEB_BASE_URL | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#public-env.vite-identity-web-base-url` |
| VITE_IDENTITY_WEB_PORT | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#public-env.vite-identity-web-port` |
| VITE_LEGAL_BASE_URL | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#public-env.vite-legal-base-url` |
| VITE_NVBES_BUILD_ID | Platform | keep | Platform lead | not-required | `deploy/oss/helm/nvbes#public-env.vite-nvbes-build-id` |
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
