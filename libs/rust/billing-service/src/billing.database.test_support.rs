//! Billing PostgreSQL test helpers.
//!
//! Dual mechanism (workspace-wide):
//! - Apps (`billing-service` / `billing-worker`): Cargo feature `database-tests`
//!   gates compilation of `#[sqlx::test]` suites. Runtime uses `DATABASE_URL`
//!   (coverage mother DB `nvbes_coverage_test`, see
//!   `scripts/test-workspace-coverage.sh` / `tools/ci/test-security-database.mjs`).
//! - Libs (`nvbes-test-utils`): isolated schemas via
//!   `NVBES_SECURITY_TEST_DATABASE_URL` without a Cargo feature.
//!
//! Prefer `#[sqlx::test(migrations = "./migrations")]` + `DATABASE_URL` for
//! app-layer persistence tests; do not invent a third harness.

use crate::config::BillingConfig;

pub fn test_config() -> BillingConfig {
    BillingConfig {
        browser_origins: Default::default(),
        public_origin: None,
        account_authority: None,
        bind_addr: "127.0.0.1:0".parse().expect("bind addr"),
        database_url: String::new(),
        stripe_secret_key: "sk_test_dummy".into(),
        stripe_webhook_secret: "whsec_fixture".into(),
        stripe_api_base_url: "https://api.stripe.com".into(),
        identity_public_key_pem: None,
        identity_token_issuer: None,
        identity_token_key_id: None,
        identity_verification_keys: "[]".into(),
        identity_resource_client_id: None,
        identity_resource_secret: None,
        metrics_token: Some("metrics_fixture".into()),
        operator_token: Some("operator_fixture".into()),
        app_url: "https://nvbes.test".into(),
        email_grpc_endpoint: None,
        email_token: None,
    }
}
