use async_graphql::{
    Context, EmptySubscription, Enum, Error, InputObject, Object, Result, Schema, SimpleObject,
};

use crate::{
    billing_client::billing_client,
    pb::nvbes::billing::v1::{
        BillingOverview as GrpcBillingOverview, BillingPortal as GrpcBillingPortal,
        CheckoutSession as GrpcCheckoutSession, CreateCheckoutRequest, GetBillingOverviewRequest,
        GetBillingPortalRequest, ProductEntitlements as GrpcProductEntitlements,
    },
    state::{GatewayRequestContext, GatewayState},
};

pub type GatewaySchema = Schema<QueryRoot, MutationRoot, EmptySubscription>;

pub fn schema(state: GatewayState) -> GatewaySchema {
    Schema::build(QueryRoot, MutationRoot, EmptySubscription)
        .data(state)
        .finish()
}

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn billing_overview(
        &self,
        ctx: &Context<'_>,
        workspace_id: String,
    ) -> Result<BillingOverview> {
        let state = ctx.data::<GatewayState>()?;
        let request_context = ctx
            .data::<GatewayRequestContext>()?
            .for_workspace(&workspace_id);
        let mut client = billing_client(&state.billing_grpc_endpoint).await?;
        let response = client
            .get_billing_overview(GetBillingOverviewRequest {
                context: Some(request_context),
                workspace_id,
            })
            .await?
            .into_inner();
        BillingOverview::from_grpc(response)
    }

    async fn billing_portal(
        &self,
        ctx: &Context<'_>,
        workspace_id: String,
    ) -> Result<BillingPortal> {
        let state = ctx.data::<GatewayState>()?;
        let request_context = ctx
            .data::<GatewayRequestContext>()?
            .for_workspace(&workspace_id);
        let mut client = billing_client(&state.billing_grpc_endpoint).await?;
        let response = client
            .get_billing_portal(GetBillingPortalRequest {
                context: Some(request_context),
                workspace_id,
            })
            .await?
            .into_inner();
        BillingPortal::from_grpc(response)
    }

    async fn billing_entitlements(
        &self,
        ctx: &Context<'_>,
        workspace_id: String,
    ) -> Result<ProductEntitlements> {
        let overview = self.billing_overview(ctx, workspace_id).await?;
        Ok(overview.entitlements)
    }
}

pub struct MutationRoot;

#[Object]
impl MutationRoot {
    async fn create_billing_checkout(
        &self,
        ctx: &Context<'_>,
        input: CreateBillingCheckoutInput,
    ) -> Result<CheckoutSession> {
        let state = ctx.data::<GatewayState>()?;
        let request_context = ctx
            .data::<GatewayRequestContext>()?
            .for_workspace(&input.workspace_id);
        let mut client = billing_client(&state.billing_grpc_endpoint).await?;
        let response = client
            .create_checkout(CreateCheckoutRequest {
                context: Some(request_context),
                workspace_id: input.workspace_id,
                plan_code: input.plan_code,
                success_url: input.success_url.unwrap_or_default(),
                cancel_url: input.cancel_url.unwrap_or_default(),
            })
            .await?
            .into_inner();
        CheckoutSession::from_grpc(response)
    }

    async fn create_billing_portal_session(
        &self,
        _ctx: &Context<'_>,
        _input: CreateBillingPortalSessionInput,
    ) -> Result<PortalSession> {
        Err(Error::new(
            "Billing gRPC CreatePortalSession is not implemented yet",
        ))
    }
}

#[derive(InputObject)]
pub struct CreateBillingCheckoutInput {
    pub workspace_id: String,
    pub plan_code: String,
    pub success_url: Option<String>,
    pub cancel_url: Option<String>,
}

#[derive(InputObject)]
pub struct CreateBillingPortalSessionInput {
    pub workspace_id: String,
    pub return_url: Option<String>,
}

#[derive(SimpleObject)]
pub struct CheckoutSession {
    pub checkout_session_id: String,
    pub checkout_url: String,
    pub provider: BillingProvider,
    pub expires_at: Option<String>,
}

impl CheckoutSession {
    fn from_grpc(value: GrpcCheckoutSession) -> Result<Self> {
        Ok(Self {
            checkout_session_id: value.checkout_session_id,
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
    fn from_grpc(value: GrpcBillingOverview) -> Result<Self> {
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
    fn from_grpc(value: GrpcBillingPortal) -> Result<Self> {
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

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum BillingProvider {
    Stripe,
    Mollie,
    Cb,
}

impl BillingProvider {
    fn parse(value: &str) -> Result<Self> {
        match value.to_ascii_lowercase().as_str() {
            "stripe" => Ok(Self::Stripe),
            "mollie" => Ok(Self::Mollie),
            "cb" => Ok(Self::Cb),
            _ => Err(Error::new(format!("unsupported billing provider: {value}"))),
        }
    }
}

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum BillingSubscriptionStatus {
    Trialing,
    Active,
    PastDue,
    Canceled,
    Incomplete,
}

impl BillingSubscriptionStatus {
    fn parse(value: &str) -> Result<Self> {
        match value.to_ascii_lowercase().as_str() {
            "trialing" => Ok(Self::Trialing),
            "active" => Ok(Self::Active),
            "past_due" => Ok(Self::PastDue),
            "canceled" => Ok(Self::Canceled),
            "incomplete" => Ok(Self::Incomplete),
            _ => Err(Error::new(format!(
                "unsupported subscription status: {value}"
            ))),
        }
    }
}

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum BillingInvoiceStatus {
    Draft,
    Open,
    Paid,
    Void,
    Uncollectible,
}

impl BillingInvoiceStatus {
    fn parse(value: &str) -> Result<Self> {
        match value.to_ascii_lowercase().as_str() {
            "draft" => Ok(Self::Draft),
            "open" => Ok(Self::Open),
            "paid" => Ok(Self::Paid),
            "void" => Ok(Self::Void),
            "uncollectible" => Ok(Self::Uncollectible),
            _ => Err(Error::new(format!("unsupported invoice status: {value}"))),
        }
    }
}

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum BillingProviderReferenceStatus {
    Active,
    Inactive,
    Failed,
}

impl BillingProviderReferenceStatus {
    fn parse(value: &str) -> Result<Self> {
        match value.to_ascii_lowercase().as_str() {
            "active" => Ok(Self::Active),
            "inactive" | "" => Ok(Self::Inactive),
            "failed" => Ok(Self::Failed),
            _ => Err(Error::new(format!(
                "unsupported provider reference status: {value}"
            ))),
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
