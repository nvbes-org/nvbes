use serde::Serialize;
use sqlx::PgPool;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::portal::PaymentMethodUpdateFlow;
use crate::provider::ProviderCode;

pub use crate::portal_views_invoices::{
    BillingPortalInvoiceProviderView, BillingPortalInvoiceView, canonical_invoice_pdf_url,
    fetch_portal_invoices,
};
pub use crate::portal_views_payment_methods::{
    BillingPortalPaymentMethodProviderView, BillingPortalPaymentMethodView,
    fetch_portal_payment_methods,
};
pub use crate::portal_views_subscriptions::{
    BillingPortalSubscriptionProviderView, fetch_portal_subscription_providers,
};

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BillingPortalCreditView {
    pub amount_minor: i64,
    pub currency: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BillingPortalView {
    pub plan_code: String,
    #[schema(value_type = ProviderCode)]
    pub provider: String,
    pub payment_method_update_flow: PaymentMethodUpdateFlow,
    pub payment_method_changes_delegated_to_provider: bool,
    pub automatically_updates_payment_method_references: bool,
    pub exposes_provider_secret_ids: bool,
    pub invoices: Vec<BillingPortalInvoiceView>,
    pub credits: Vec<BillingPortalCreditView>,
    pub payment_methods: Vec<BillingPortalPaymentMethodView>,
    pub subscriptions: Vec<BillingPortalSubscriptionProviderView>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BillingPortalCapabilities {
    pub exposes_provider_secret_ids: bool,
    pub payment_method_update_flow: PaymentMethodUpdateFlow,
    pub payment_method_changes_delegated_to_provider: bool,
    pub automatically_updates_payment_method_references: bool,
    pub shows_canonical_invoices: bool,
    pub shows_credits: bool,
}

pub fn billing_portal_capabilities() -> BillingPortalCapabilities {
    BillingPortalCapabilities {
        exposes_provider_secret_ids: false,
        payment_method_update_flow: PaymentMethodUpdateFlow::NvbesProviderRedirect,
        payment_method_changes_delegated_to_provider: false,
        automatically_updates_payment_method_references: false,
        shows_canonical_invoices: true,
        shows_credits: true,
    }
}

pub async fn fetch_portal_view(
    db: &PgPool,
    workspace_id: Uuid,
) -> Result<BillingPortalView, sqlx::Error> {
    let billing = crate::fetch_workspace_billing_overview(db, workspace_id).await?;
    let invoices = fetch_portal_invoices(db, workspace_id).await?;
    let credits = fetch_portal_credits(db, workspace_id).await?;
    let payment_methods = fetch_portal_payment_methods(db, workspace_id).await?;
    let subscriptions = fetch_portal_subscription_providers(db, workspace_id).await?;

    Ok(BillingPortalView {
        plan_code: billing.plan.code,
        payment_method_update_flow: PaymentMethodUpdateFlow::for_provider_code(
            &billing.subscription.billing_provider,
        ),
        provider: billing.subscription.billing_provider,
        payment_method_changes_delegated_to_provider: false,
        automatically_updates_payment_method_references: false,
        exposes_provider_secret_ids: false,
        invoices,
        credits,
        payment_methods,
        subscriptions,
    })
}

async fn fetch_portal_credits(
    db: &PgPool,
    workspace_id: Uuid,
) -> Result<Vec<BillingPortalCreditView>, sqlx::Error> {
    let credits = sqlx::query_as::<_, BillingPortalCreditRecord>(
        r#"
        SELECT COALESCE(SUM(cc.remaining_minor), 0)::bigint AS amount_minor,
               cc.currency::text AS currency
        FROM billing_commercial_credits cc
        JOIN workspaces w ON w.tenant_id = cc.tenant_id
        WHERE w.id = $1
          AND cc.remaining_minor > 0
          AND (cc.expires_at IS NULL OR cc.expires_at > NOW())
        GROUP BY cc.currency
        ORDER BY cc.currency
        "#,
    )
    .bind(workspace_id)
    .fetch_all(db)
    .await?;

    Ok(credits.into_iter().map(Into::into).collect())
}

#[derive(Debug, sqlx::FromRow)]
struct BillingPortalCreditRecord {
    amount_minor: i64,
    currency: String,
}

impl From<BillingPortalCreditRecord> for BillingPortalCreditView {
    fn from(record: BillingPortalCreditRecord) -> Self {
        Self {
            amount_minor: record.amount_minor,
            currency: record.currency,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn billing_portal_capabilities_do_not_expose_provider_secret_ids() {
        let capabilities = billing_portal_capabilities();
        assert!(!capabilities.exposes_provider_secret_ids);
        assert_eq!(
            capabilities.payment_method_update_flow,
            PaymentMethodUpdateFlow::NvbesProviderRedirect
        );
        assert!(!capabilities.payment_method_changes_delegated_to_provider);
        assert!(!capabilities.automatically_updates_payment_method_references);
    }

    #[test]
    fn billing_portal_view_contains_no_provider_secret_identifiers() {
        let view = BillingPortalView {
            plan_code: "team".to_string(),
            provider: "stripe".to_string(),
            payment_method_update_flow: PaymentMethodUpdateFlow::NvbesProviderRedirect,
            payment_method_changes_delegated_to_provider: false,
            automatically_updates_payment_method_references: false,
            exposes_provider_secret_ids: false,
            invoices: vec![BillingPortalInvoiceView {
                invoice_id: Uuid::nil(),
                invoice_number: Some("NVBES-2026-000001".to_string()),
                canonical_pdf_url: canonical_invoice_pdf_url(Uuid::nil(), Uuid::nil()),
                status: "issued".to_string(),
                total_minor: 4_680,
                currency: "EUR".to_string(),
                issued_at: None,
                due_at: None,
                paid_at: None,
                providers: vec![BillingPortalInvoiceProviderView {
                    provider: "stripe".to_string(),
                    status: "active".to_string(),
                    invoice_number: Some("INV-2026-000001".to_string()),
                    pdf_available: true,
                }],
            }],
            credits: vec![BillingPortalCreditView {
                amount_minor: 1_000,
                currency: "EUR".to_string(),
            }],
            payment_methods: vec![BillingPortalPaymentMethodView {
                payment_method_id: Uuid::nil(),
                method_type: "card".to_string(),
                display_label: Some("Visa **** 4242".to_string()),
                brand: Some("visa".to_string()),
                last4: Some("4242".to_string()),
                exp_month: Some(12),
                exp_year: Some(2028),
                funding: Some("credit".to_string()),
                issuer_country: Some("FR".to_string()),
                status: "active".to_string(),
                is_primary: true,
                providers: vec![BillingPortalPaymentMethodProviderView {
                    provider: "mollie".to_string(),
                    status: "active".to_string(),
                    mandate_status: "valid".to_string(),
                    reusable: true,
                }],
            }],
            subscriptions: vec![BillingPortalSubscriptionProviderView {
                provider: "mollie".to_string(),
                status: "active".to_string(),
                primary: true,
                fallback_eligible: false,
                activated_at: None,
                deactivated_at: None,
            }],
        };

        assert!(!view.exposes_provider_secret_ids);
        assert_eq!(view.payment_methods[0].last4.as_deref(), Some("4242"));
        assert_eq!(view.invoices[0].providers[0].provider, "stripe");
        assert_eq!(view.subscriptions[0].provider, "mollie");
    }
}
