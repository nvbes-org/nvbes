use nvbes_core::config::AppConfig;
use uuid::Uuid;

use crate::models::BillingStateRecord;
use crate::types::CreatePortalInput;

use super::{PortalSessionError, create_provider_portal_session};

fn billing_record(provider: &str, stripe_customer_id: Option<String>) -> BillingStateRecord {
    BillingStateRecord {
        workspace_id: Uuid::new_v4(),
        workspace_name: "Acme".to_string(),
        owner_principal_id: Uuid::new_v4(),
        owner_email: "owner@example.com".to_string(),
        trial_ends_at: None,
        plan_id: Uuid::new_v4(),
        plan_code: "team".to_string(),
        included_storage_gb: 10,
        included_users: 2,
        retention_days: 90,
        max_share_links: 25,
        audit_level: "standard".to_string(),
        max_share_link_ttl_days: 30,
        subscription_status: "active".to_string(),
        billing_provider: provider.to_string(),
        billing_customer_id: None,
        billing_subscription_id: None,
        current_period_start: None,
        current_period_end: None,
        provider_customer_id: None,
        stripe_customer_id,
        billing_email: None,
        country: None,
        customer_type: "b2c".to_string(),
        vat_number: None,
        tax_exempt_status: None,
        used_storage_bytes: 0,
        bandwidth_out_bytes_month: 0,
        active_user_count: 1,
    }
}

#[tokio::test]
async fn create_provider_portal_session_rejects_unknown_and_unsupported_providers() {
    let config = AppConfig::default();
    let input = CreatePortalInput { return_url: None };

    let unknown = billing_record("unknown", Some("cus_test".into()));
    assert!(matches!(
        create_provider_portal_session(&config, &unknown, input.clone()).await,
        Err(PortalSessionError::UnknownBillingProvider)
    ));

    let mollie = billing_record("mollie", Some("cus_mollie".into()));
    assert!(matches!(
        create_provider_portal_session(&config, &mollie, input.clone()).await,
        Err(PortalSessionError::ProviderPortalUnavailable)
    ));
}

#[tokio::test]
async fn create_provider_portal_session_requires_stripe_customer_id() {
    let config = AppConfig::default();
    let record = billing_record("stripe", None);
    let input = CreatePortalInput { return_url: None };

    assert!(matches!(
        create_provider_portal_session(&config, &record, input).await,
        Err(PortalSessionError::MissingBillingCustomer)
    ));
}
