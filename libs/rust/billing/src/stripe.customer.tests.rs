use uuid::Uuid;

use super::{build_customer_fields, create_stripe_customer};
use crate::models::BillingStateRecord;
use crate::stripe::StripeProviderError;
use nvbes_core::config::AppConfig;

fn sample_record() -> BillingStateRecord {
    BillingStateRecord {
        workspace_id: Uuid::from_u128(7),
        workspace_name: "Acme".to_string(),
        owner_principal_id: Uuid::from_u128(9),
        owner_email: "owner@example.com".to_string(),
        trial_ends_at: None,
        plan_id: Uuid::nil(),
        plan_code: "team".to_string(),
        included_storage_gb: 10,
        included_users: 2,
        retention_days: 90,
        max_share_links: 25,
        audit_level: "standard".to_string(),
        max_share_link_ttl_days: 30,
        subscription_status: "active".to_string(),
        billing_provider: "stripe".to_string(),
        billing_customer_id: None,
        billing_subscription_id: None,
        current_period_start: None,
        current_period_end: None,
        provider_customer_id: None,
        stripe_customer_id: None,
        billing_email: None,
        country: None,
        customer_type: "b2b".to_string(),
        vat_number: None,
        tax_exempt_status: None,
        used_storage_bytes: 0,
        bandwidth_out_bytes_month: 0,
        active_user_count: 1,
    }
}

#[test]
fn build_customer_fields_maps_workspace_and_owner_metadata() {
    let record = sample_record();
    let fields = build_customer_fields(&record);
    let map = fields
        .into_iter()
        .collect::<std::collections::HashMap<_, _>>();

    assert_eq!(map.get("email"), Some(&"owner@example.com".to_string()));
    assert_eq!(map.get("name"), Some(&"Acme".to_string()));
    assert_eq!(
        map.get("metadata[workspace_id]"),
        Some(&Uuid::from_u128(7).to_string())
    );
    assert_eq!(
        map.get("metadata[owner_principal_id]"),
        Some(&Uuid::from_u128(9).to_string())
    );
}

#[tokio::test]
async fn create_stripe_customer_requires_configuration() {
    let config = AppConfig {
        stripe_secret_key: None,
        ..AppConfig::default()
    };
    let err = create_stripe_customer(&config, &sample_record())
        .await
        .unwrap_err();
    assert!(matches!(err, StripeProviderError::NotConfigured));
}
