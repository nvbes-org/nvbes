use async_graphql::{Error, Result, SimpleObject};

use crate::{
    pb::nvbes::billing::v1::{
        BillingOverview as GrpcBillingOverview, BillingPortal as GrpcBillingPortal,
        CheckoutSession as GrpcCheckoutSession, ProductEntitlements as GrpcProductEntitlements,
    },
    schema_enums::{
        BillingInvoiceStatus, BillingProvider, BillingProviderReferenceStatus,
        BillingSubscriptionStatus,
    },
};

#[derive(SimpleObject)]
pub struct CheckoutSession {
    pub checkout_url: String,
    pub provider: BillingProvider,
    pub expires_at: Option<String>,
}

impl CheckoutSession {
    pub fn from_grpc(value: GrpcCheckoutSession) -> Result<Self> {
        Ok(Self {
            checkout_url: value.checkout_url,
            provider: BillingProvider::parse(&value.provider)?,
            expires_at: empty_to_none(value.expires_at),
        })
    }
}

#[derive(SimpleObject)]
pub struct PortalSession {
    pub portal_session_id: String,
    pub portal_url: String,
    pub provider: BillingProvider,
}

#[derive(SimpleObject)]
pub struct BillingOverview {
    pub workspace_id: String,
    pub plan_code: String,
    pub subscription_status: BillingSubscriptionStatus,
    pub billing_provider: BillingProvider,
    pub current_period_start: Option<String>,
    pub current_period_end: Option<String>,
    pub entitlements: ProductEntitlements,
}

impl BillingOverview {
    pub fn from_grpc(value: GrpcBillingOverview) -> Result<Self> {
        Ok(Self {
            workspace_id: value.workspace_id,
            plan_code: value.plan_code,
            subscription_status: BillingSubscriptionStatus::parse(&value.subscription_status)?,
            billing_provider: BillingProvider::parse(&value.billing_provider)?,
            current_period_start: empty_to_none(value.current_period_start),
            current_period_end: empty_to_none(value.current_period_end),
            entitlements: ProductEntitlements::from_grpc(value.entitlements.unwrap_or_default()),
        })
    }
}

#[derive(SimpleObject)]
pub struct BillingPortal {
    pub workspace_id: String,
    pub invoices: Vec<BillingInvoice>,
    pub payment_methods: Vec<BillingPaymentMethod>,
    pub subscriptions: Vec<BillingSubscription>,
}

impl BillingPortal {
    pub fn from_grpc(value: GrpcBillingPortal) -> Result<Self> {
        Ok(Self {
            workspace_id: value.workspace_id,
            invoices: value
                .invoices
                .into_iter()
                .map(BillingInvoice::from_grpc)
                .collect::<Result<Vec<_>>>()?,
            payment_methods: value
                .payment_methods
                .into_iter()
                .map(BillingPaymentMethod::from_grpc)
                .collect::<Result<Vec<_>>>()?,
            subscriptions: value
                .subscriptions
                .into_iter()
                .map(BillingSubscription::from_grpc)
                .collect::<Result<Vec<_>>>()?,
        })
    }
}

#[derive(SimpleObject)]
pub struct BillingInvoice {
    pub invoice_id: String,
    pub invoice_number: String,
    pub status: BillingInvoiceStatus,
    pub total_minor: i32,
    pub currency: String,
    pub issued_at: Option<String>,
    pub pdf_path: Option<String>,
    pub providers: Vec<BillingProviderReference>,
}

impl BillingInvoice {
    fn from_grpc(value: crate::pb::nvbes::billing::v1::BillingInvoice) -> Result<Self> {
        Ok(Self {
            invoice_id: value.invoice_id,
            invoice_number: value.invoice_number,
            status: BillingInvoiceStatus::parse(&value.status)?,
            total_minor: checked_i32(value.total_minor, "invoice total_minor")?,
            currency: value.currency,
            issued_at: empty_to_none(value.issued_at),
            pdf_path: None,
            providers: value
                .providers
                .into_iter()
                .map(BillingProviderReference::from_grpc)
                .collect::<Result<Vec<_>>>()?,
        })
    }
}

#[derive(SimpleObject)]
pub struct BillingPaymentMethod {
    pub payment_method_id: String,
    pub brand: String,
    pub last4: String,
    pub exp_month: i32,
    pub exp_year: i32,
    pub providers: Vec<BillingProviderReference>,
}

impl BillingPaymentMethod {
    fn from_grpc(value: crate::pb::nvbes::billing::v1::BillingPaymentMethod) -> Result<Self> {
        Ok(Self {
            payment_method_id: value.payment_method_id,
            brand: value.brand,
            last4: value.last4,
            exp_month: value.exp_month,
            exp_year: value.exp_year,
            providers: value
                .providers
                .into_iter()
                .map(BillingProviderReference::from_grpc)
                .collect::<Result<Vec<_>>>()?,
        })
    }
}

#[derive(SimpleObject)]
pub struct BillingSubscription {
    pub subscription_id: String,
    pub plan_code: String,
    pub status: BillingSubscriptionStatus,
    pub providers: Vec<BillingProviderReference>,
}

impl BillingSubscription {
    fn from_grpc(value: crate::pb::nvbes::billing::v1::BillingSubscription) -> Result<Self> {
        Ok(Self {
            subscription_id: value.subscription_id,
            plan_code: value.plan_code,
            status: BillingSubscriptionStatus::parse(&value.status)?,
            providers: value
                .providers
                .into_iter()
                .map(BillingProviderReference::from_grpc)
                .collect::<Result<Vec<_>>>()?,
        })
    }
}

#[derive(SimpleObject)]
pub struct BillingProviderReference {
    pub provider: BillingProvider,
    pub status: BillingProviderReferenceStatus,
    pub primary: bool,
    pub fallback_eligible: bool,
}

impl BillingProviderReference {
    fn from_grpc(value: crate::pb::nvbes::billing::v1::BillingProviderReference) -> Result<Self> {
        Ok(Self {
            provider: BillingProvider::parse(&value.provider)?,
            status: BillingProviderReferenceStatus::parse(&value.status)?,
            primary: value.primary,
            fallback_eligible: value.fallback_eligible,
        })
    }
}

#[derive(SimpleObject)]
pub struct ProductEntitlements {
    pub included_storage_gb: i32,
    pub included_users: i32,
    pub retention_days: i32,
    pub max_share_links: i32,
    pub audit_level: String,
}

impl ProductEntitlements {
    fn from_grpc(value: GrpcProductEntitlements) -> Self {
        Self {
            included_storage_gb: i64_to_i32_saturating(value.included_storage_gb),
            included_users: i64_to_i32_saturating(value.included_users),
            retention_days: i64_to_i32_saturating(value.retention_days),
            max_share_links: i64_to_i32_saturating(value.max_share_links),
            audit_level: value.audit_level,
        }
    }
}

fn empty_to_none(value: String) -> Option<String> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}

fn checked_i32(value: i64, field: &str) -> Result<i32> {
    i32::try_from(value).map_err(|_| Error::new(format!("{field} exceeds GraphQL Int range")))
}

fn i64_to_i32_saturating(value: i64) -> i32 {
    i32::try_from(value).unwrap_or(if value.is_negative() {
        i32::MIN
    } else {
        i32::MAX
    })
}
