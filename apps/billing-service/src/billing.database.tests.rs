use uuid::Uuid;

use crate::{config::BillingConfig, synthetic};
use super::{connect, migrate};

#[tokio::test]
async fn billing_lifecycle_is_isolated_deduplicated_and_audited() {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL is required");
    let pool = connect(&database_url, 2)
        .await
        .expect("test database connects");
    migrate(&pool).await.expect("billing migrations apply");

    let config = BillingConfig {
        bind_addr: "127.0.0.1:8080".parse().unwrap(),
        database_url: database_url.clone(),
        stripe_secret_key: "sk_test_mock_for_db_tests".into(),
        stripe_webhook_secret: "whsec_mock_secret".into(),
        stripe_api_base_url: "https://api.stripe.com".into(),
        identity_public_key_pem: None,
        metrics_token: None,
        operator_token: Some("test_operator_token".into()),
        app_url: "https://nvbes.test".into(),
    };

    let workspace_id = Uuid::new_v4();
    let owner_id = Uuid::new_v4();

    let result = synthetic::run(&pool, &config, workspace_id, owner_id)
        .await
        .expect("synthetic Billing lifecycle succeeds");

    assert!(result.checkout_idempotent);
    assert!(result.webhook_deduplicated);
    assert!(result.out_of_order_protected);
    assert!(result.reconciliation_resolved);
    assert_eq!(result.subscription_status, "active");
    assert!(result.audit_events >= 2);
    assert!(result.outbox_events >= 1);
}
