use crate::models::BillingStateRecord;
use crate::provider::{
    ProviderCode, provider_code, provider_codes, provider_customer_id_for,
    provider_supports_external_portal,
};
use uuid::Uuid;

fn billing_record(provider: &str) -> BillingStateRecord {
    BillingStateRecord {
        workspace_id: Uuid::nil(),
        workspace_name: "Acme".to_string(),
        owner_principal_id: Uuid::nil(),
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
        billing_provider: provider.to_string(),
        billing_customer_id: Some(format!("{provider}_billing")),
        billing_subscription_id: None,
        current_period_start: None,
        current_period_end: None,
        provider_customer_id: Some(format!("{provider}_provider")),
        stripe_customer_id: Some("stripe_legacy".to_string()),
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
fn provider_customer_id_for_prefers_legacy_stripe_id_for_stripe() {
    let record = billing_record("mollie");

    assert_eq!(
        provider_customer_id_for(&record, ProviderCode::Stripe).as_deref(),
        Some("stripe_legacy")
    );
}

#[test]
fn provider_customer_id_for_uses_current_provider_customer() {
    let record = billing_record("mollie");

    assert_eq!(
        provider_customer_id_for(&record, ProviderCode::Mollie).as_deref(),
        Some("mollie_provider")
    );
}

#[test]
fn provider_customer_id_for_does_not_reuse_other_provider_customer() {
    let mut record = billing_record("mollie");
    record.stripe_customer_id = None;

    assert_eq!(
        provider_customer_id_for(&record, ProviderCode::Stripe),
        None
    );
}

#[test]
fn provider_code_parses_supported_provider_codes() {
    assert_eq!(provider_code("stripe"), Some(ProviderCode::Stripe));
    assert_eq!(provider_code("mollie"), Some(ProviderCode::Mollie));
    assert_eq!(provider_code("cb"), Some(ProviderCode::Cb));
    assert_eq!(provider_code("unknown"), None);
}

#[test]
fn provider_codes_lists_supported_public_codes() {
    assert_eq!(provider_codes(), ["stripe", "mollie", "cb"]);
}

#[test]
fn provider_code_exposes_public_code() {
    assert_eq!(ProviderCode::Stripe.as_str(), "stripe");
    assert_eq!(ProviderCode::Mollie.as_str(), "mollie");
    assert_eq!(ProviderCode::Cb.as_str(), "cb");
}

#[test]
fn provider_code_serializes_as_public_code() {
    assert_eq!(
        serde_json::to_value(ProviderCode::Stripe).expect("provider code should serialize"),
        serde_json::json!("stripe")
    );
    assert_eq!(
        serde_json::from_value::<ProviderCode>(serde_json::json!("mollie"))
            .expect("provider code should deserialize"),
        ProviderCode::Mollie
    );
    assert_eq!(
        serde_json::from_value::<ProviderCode>(serde_json::json!("cb"))
            .expect("provider code should deserialize"),
        ProviderCode::Cb
    );
}

#[test]
fn provider_supports_external_portal_only_for_stripe() {
    assert!(provider_supports_external_portal(ProviderCode::Stripe));
    assert!(!provider_supports_external_portal(ProviderCode::Mollie));
    assert!(!provider_supports_external_portal(ProviderCode::Cb));
}
