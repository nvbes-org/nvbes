use crate::models::BillingStateRecord;
use crate::shared::{
    EUR, EXTRA_SEAT_CENTS_PER_MONTH, STORAGE_OVERAGE_CENTS_PER_GB_MONTH, api_key_limit,
    current_billing_period, div_ceil, plan_monthly_price_cents,
};
use crate::types::{
    BillingAccountView, InvoiceEstimateView, PlanView, ProductEntitlementsView, SubscriptionView,
};

pub fn build_invoice_estimate(record: &BillingStateRecord) -> InvoiceEstimateView {
    let (billing_period_start, billing_period_end) = current_billing_period();
    let included_storage_bytes = i64::from(record.included_storage_gb) * 1024 * 1024 * 1024;
    let storage_overage_bytes = record
        .used_storage_bytes
        .saturating_sub(included_storage_bytes);
    let storage_overage_gb = div_ceil(storage_overage_bytes, 1024 * 1024 * 1024);
    let storage_overage_amount_cents = storage_overage_gb * STORAGE_OVERAGE_CENTS_PER_GB_MONTH;
    let seat_overage = record
        .active_user_count
        .saturating_sub(i64::from(record.included_users));
    let seat_overage_amount_cents = seat_overage * EXTRA_SEAT_CENTS_PER_MONTH;
    let base_amount_cents = plan_monthly_price_cents(&record.plan_code);
    let estimated_amount_cents =
        base_amount_cents + storage_overage_amount_cents + seat_overage_amount_cents;

    InvoiceEstimateView {
        workspace_id: record.workspace_id,
        billing_period_start,
        billing_period_end,
        base_amount_cents,
        storage_overage_amount_cents,
        seat_overage_amount_cents,
        estimated_amount_cents,
        currency: EUR.to_string(),
    }
}

pub fn plan_view(record: &BillingStateRecord) -> PlanView {
    PlanView {
        code: record.plan_code.clone(),
        included_storage_gb: record.included_storage_gb,
        included_users: record.included_users,
        retention_days: record.retention_days,
        max_share_links: record.max_share_links,
        audit_level: record.audit_level.clone(),
        max_share_link_ttl_days: record.max_share_link_ttl_days,
        monthly_price_cents: plan_monthly_price_cents(&record.plan_code),
        currency: EUR.to_string(),
    }
}

pub fn subscription_view(record: &BillingStateRecord) -> SubscriptionView {
    SubscriptionView {
        status: record.subscription_status.clone(),
        billing_provider: "stripe".to_string(),
        billing_customer_id: record.billing_customer_id.clone(),
        billing_subscription_id: record.billing_subscription_id.clone(),
        current_period_start: record.current_period_start,
        current_period_end: record.current_period_end,
        trial_ends_at: record.trial_ends_at,
    }
}

pub fn billing_account_view(record: &BillingStateRecord) -> BillingAccountView {
    BillingAccountView {
        stripe_customer_id: record.stripe_customer_id.clone(),
        billing_email: record.billing_email.clone(),
        country: record.country.clone(),
        customer_type: record.customer_type.clone(),
        vat_number: record.vat_number.clone(),
        tax_exempt_status: record.tax_exempt_status.clone(),
    }
}

pub fn entitlements_view(record: &BillingStateRecord) -> ProductEntitlementsView {
    let billing_locked = matches!(
        record.subscription_status.as_str(),
        "past_due" | "canceled" | "incomplete" | "suspended"
    );

    ProductEntitlementsView {
        can_upload: !billing_locked,
        can_create_share_links: !billing_locked && record.max_share_links > 0,
        included_storage_bytes: i64::from(record.included_storage_gb) * 1024 * 1024 * 1024,
        included_users: record.included_users,
        max_share_links: record.max_share_links,
        max_share_link_ttl_days: record.max_share_link_ttl_days,
        audit_level: record.audit_level.clone(),
        api_key_limit: api_key_limit(&record.plan_code),
        billing_locked,
    }
}
