use super::{build_checkout_session_fields, build_customer_fields};
use crate::domains::billing::types::BillingStateRecord;
use uuid::Uuid;

#[test]
fn build_checkout_session_fields_includes_workspace_and_subscription_metadata() {
    let workspace_id = Uuid::new_v4();
    let owner_principal_id = Uuid::new_v4();
    let fields = build_checkout_session_fields(
        "cus_123",
        owner_principal_id,
        workspace_id,
        "pro",
        "price_123",
        "https://app.example.com/billing/success",
        "https://app.example.com/billing/cancel",
    );

    assert!(
        fields
            .iter()
            .any(|(key, value)| key == "client_reference_id" && value == &workspace_id.to_string())
    );
    assert!(
        fields
            .iter()
            .any(|(key, value)| key == "metadata[workspace_id]"
                && value == &workspace_id.to_string())
    );
    assert!(fields.iter().any(|(key, value)| {
        key == "metadata[owner_principal_id]" && value == &owner_principal_id.to_string()
    }));
    assert!(fields.iter().any(|(key, value)| {
        key == "subscription_data[metadata][workspace_id]" && value == &workspace_id.to_string()
    }));
    assert!(fields.iter().any(|(key, value)| {
        key == "subscription_data[metadata][owner_principal_id]"
            && value == &owner_principal_id.to_string()
    }));
}

#[test]
fn build_customer_fields_includes_workspace_metadata() {
    let record = BillingStateRecord {
        workspace_id: Uuid::new_v4(),
        workspace_name: "Acme".to_string(),
        owner_principal_id: Uuid::new_v4(),
        owner_email: "owner@example.com".to_string(),
        trial_ends_at: None,
        plan_code: "trial".to_string(),
        included_storage_gb: 0,
        included_users: 0,
        retention_days: 30,
        max_share_links: 100,
        audit_level: "standard".to_string(),
        max_share_link_ttl_days: 90,
        subscription_status: "trialing".to_string(),
        billing_customer_id: None,
        billing_subscription_id: None,
        current_period_start: None,
        current_period_end: None,
        stripe_customer_id: None,
        billing_email: None,
        country: None,
        customer_type: "b2b".to_string(),
        vat_number: None,
        tax_exempt_status: None,
        used_storage_bytes: 0,
        bandwidth_out_bytes_month: 0,
        active_user_count: 0,
    };

    let fields = build_customer_fields(&record);

    assert!(
        fields
            .iter()
            .any(|(key, value)| key == "metadata[workspace_id]"
                && value == &record.workspace_id.to_string())
    );
    assert!(fields.iter().any(|(key, value)| {
        key == "metadata[owner_principal_id]" && value == &record.owner_principal_id.to_string()
    }));
}
