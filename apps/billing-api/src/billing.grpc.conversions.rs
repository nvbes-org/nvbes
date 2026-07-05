use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::grpc::pb::nvbes::billing::v1::{
    BillingInvoice, BillingOverview, BillingPaymentMethod, BillingPortal, BillingProviderReference,
    BillingSubscription, CheckoutSession, ProductEntitlements, ReconciliationRun,
};

pub fn billing_overview_response(
    value: nvbes_billing::types::BillingOverviewResponse,
) -> BillingOverview {
    BillingOverview {
        workspace_id: value.workspace_id.to_string(),
        plan_code: value.plan.code,
        subscription_status: value.subscription.status,
        billing_provider: value.subscription.billing_provider,
        current_period_start: optional_datetime(value.subscription.current_period_start),
        current_period_end: optional_datetime(value.subscription.current_period_end),
        entitlements: Some(ProductEntitlements {
            included_storage_gb: value.entitlements.included_storage_bytes / 1_073_741_824,
            included_users: i64::from(value.entitlements.included_users),
            retention_days: i64::from(value.plan.retention_days),
            max_share_links: i64::from(value.entitlements.max_share_links),
            audit_level: value.entitlements.audit_level,
        }),
    }
}

pub fn billing_portal_view(
    workspace_id: Uuid,
    value: nvbes_billing::portal_views::BillingPortalView,
) -> BillingPortal {
    BillingPortal {
        workspace_id: workspace_id.to_string(),
        invoices: value.invoices.into_iter().map(billing_invoice).collect(),
        payment_methods: value
            .payment_methods
            .into_iter()
            .map(billing_payment_method)
            .collect(),
        subscriptions: value
            .subscriptions
            .into_iter()
            .map(billing_subscription)
            .collect(),
    }
}

pub fn checkout_session_response(
    value: nvbes_billing::types::CheckoutSessionResponse,
) -> CheckoutSession {
    CheckoutSession {
        checkout_session_id: value.checkout_id,
        checkout_url: value.url,
        provider: value.provider,
        expires_at: String::new(),
    }
}

pub fn reconciliation_run(
    value: nvbes_billing::reconciliation_db::BillingReconciliationRunResult,
) -> ReconciliationRun {
    ReconciliationRun {
        run_id: value.run_id.to_string(),
        period_start: value.period_start.to_rfc3339(),
        period_end: value.period_end.to_rfc3339(),
        differences_created: value.differences_created,
    }
}

fn billing_invoice(value: nvbes_billing::portal_views::BillingPortalInvoiceView) -> BillingInvoice {
    BillingInvoice {
        invoice_id: value.invoice_id.to_string(),
        invoice_number: value.invoice_number.unwrap_or_default(),
        status: value.status,
        total_minor: value.total_minor,
        currency: value.currency,
        issued_at: optional_datetime(value.issued_at),
        providers: value.providers.into_iter().map(invoice_provider).collect(),
    }
}

fn billing_payment_method(
    value: nvbes_billing::portal_views::BillingPortalPaymentMethodView,
) -> BillingPaymentMethod {
    BillingPaymentMethod {
        payment_method_id: value.payment_method_id.to_string(),
        brand: value.brand.unwrap_or_default(),
        last4: value.last4.unwrap_or_default(),
        exp_month: i32::from(value.exp_month.unwrap_or_default()),
        exp_year: i32::from(value.exp_year.unwrap_or_default()),
        providers: value
            .providers
            .into_iter()
            .map(|provider| BillingProviderReference {
                provider: provider.provider,
                status: provider.status,
                primary: value.is_primary,
                fallback_eligible: provider.reusable,
            })
            .collect(),
    }
}

fn billing_subscription(
    value: nvbes_billing::portal_views::BillingPortalSubscriptionProviderView,
) -> BillingSubscription {
    BillingSubscription {
        subscription_id: stable_subscription_reference(&value.provider, &value.status),
        plan_code: String::new(),
        status: value.status,
        providers: vec![BillingProviderReference {
            provider: value.provider,
            status: String::new(),
            primary: value.primary,
            fallback_eligible: value.fallback_eligible,
        }],
    }
}

fn invoice_provider(
    value: nvbes_billing::portal_views::BillingPortalInvoiceProviderView,
) -> BillingProviderReference {
    BillingProviderReference {
        provider: value.provider,
        status: value.status,
        primary: false,
        fallback_eligible: false,
    }
}

fn stable_subscription_reference(provider: &str, status: &str) -> String {
    format!("provider:{provider}:status:{status}")
}

fn optional_datetime(value: Option<DateTime<Utc>>) -> String {
    value.map_or_else(String::new, |value| value.to_rfc3339())
}

#[cfg(test)]
mod tests {
    use super::checkout_session_response;

    #[test]
    fn checkout_response_uses_local_checkout_id_not_provider_session_id() {
        let response =
            checkout_session_response(nvbes_billing::types::CheckoutSessionResponse {
                provider: "stripe".to_string(),
                session_id: "cs_provider_secret".to_string(),
                checkout_id: "local_checkout_123".to_string(),
                url: "https://checkout.example".to_string(),
                provider_customer_id: "cus_provider_secret".to_string(),
                provider_product_id: None,
                provider_price_id: None,
                payment_id: None,
                stripe_customer_id: None,
                stripe_price_id: None,
            });

        assert_eq!(response.checkout_session_id, "local_checkout_123");
        assert!(!response.checkout_session_id.contains("provider_secret"));
    }
}
