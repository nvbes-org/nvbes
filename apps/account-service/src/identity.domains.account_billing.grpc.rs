use axum::http::{HeaderMap, StatusCode, header};
use tonic::{Code, transport::Channel};
use uuid::Uuid;

use crate::{
    domains::authz::WorkspaceAccess,
    grpc_pb::nvbes::{
        billing::v1::{
            BillingOverview as GrpcBillingOverview, BillingPortal as GrpcBillingPortal,
            CheckoutSession, PortalSession, billing_service_client::BillingServiceClient,
        },
        platform::v1::{RequestContext, TenantContext},
    },
    http::error::AppError,
};

use super::types::{
    AccountBillingEntitlements, AccountBillingInvoice, AccountBillingOverview,
    AccountBillingPaymentMethod, AccountBillingPortalView, AccountBillingProviderReference,
    AccountBillingSession, AccountBillingSubscription,
};

const BILLING_GRPC_ENDPOINT_ENV: &str = "NVBES_BILLING_GRPC_ENDPOINT";

pub fn billing_grpc_endpoint(default_api_port: u16) -> anyhow::Result<String> {
    match std::env::var(BILLING_GRPC_ENDPOINT_ENV) {
        Ok(endpoint) if !endpoint.trim().is_empty() => Ok(endpoint),
        Ok(_) => anyhow::bail!("{BILLING_GRPC_ENDPOINT_ENV} must not be empty"),
        Err(std::env::VarError::NotPresent) => {
            let port = default_api_port
                .checked_add(21)
                .ok_or_else(|| anyhow::anyhow!("Default Billing gRPC port overflowed"))?;
            Ok(format!("http://127.0.0.1:{port}"))
        }
        Err(error) => Err(anyhow::anyhow!(
            "{BILLING_GRPC_ENDPOINT_ENV} could not be read: {error}"
        )),
    }
}

pub async fn billing_client(endpoint: &str) -> Result<BillingServiceClient<Channel>, AppError> {
    BillingServiceClient::connect(endpoint.to_string())
        .await
        .map_err(|error| AppError::internal("billing_grpc_connect_failed", error.to_string()))
}

pub fn request_context(headers: &HeaderMap, access: &WorkspaceAccess) -> RequestContext {
    let request_id = header_value(headers, "x-request-id")
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let correlation_id = header_value(headers, "x-correlation-id")
        .unwrap_or(request_id.as_str())
        .to_string();

    RequestContext {
        request_id,
        correlation_id,
        actor_principal_id: access.auth.user_id.to_string(),
        tenant: Some(TenantContext {
            tenant_id: access
                .tenant_id
                .map_or_else(String::new, |id| id.to_string()),
            workspace_id: access.workspace_id.to_string(),
            region_id: access.auth.workspace_region.clone().unwrap_or_default(),
            data_residency: access.auth.workspace_region.clone().unwrap_or_default(),
        }),
    }
}

pub fn overview_from_grpc(value: GrpcBillingOverview) -> AccountBillingOverview {
    let entitlements = value.entitlements.unwrap_or_default();
    AccountBillingOverview {
        workspace_id: value.workspace_id,
        plan_code: value.plan_code,
        subscription_status: value.subscription_status,
        billing_provider: value.billing_provider,
        current_period_start: empty_to_none(value.current_period_start),
        current_period_end: empty_to_none(value.current_period_end),
        entitlements: AccountBillingEntitlements {
            included_storage_gb: entitlements.included_storage_gb,
            included_users: entitlements.included_users,
            retention_days: entitlements.retention_days,
            max_share_links: entitlements.max_share_links,
            audit_level: entitlements.audit_level,
        },
    }
}

pub fn portal_from_grpc(value: GrpcBillingPortal) -> AccountBillingPortalView {
    let provider = value
        .subscriptions
        .iter()
        .flat_map(|subscription| subscription.providers.iter())
        .find(|provider| provider.primary)
        .or_else(|| {
            value
                .subscriptions
                .iter()
                .flat_map(|subscription| subscription.providers.iter())
                .next()
        })
        .map_or_else(String::new, |provider| provider.provider.clone());

    AccountBillingPortalView {
        workspace_id: value.workspace_id,
        provider,
        invoices: value.invoices.into_iter().map(invoice_from_grpc).collect(),
        payment_methods: value
            .payment_methods
            .into_iter()
            .map(payment_method_from_grpc)
            .collect(),
        subscriptions: value
            .subscriptions
            .into_iter()
            .map(subscription_from_grpc)
            .collect(),
    }
}

pub fn checkout_session_from_grpc(value: CheckoutSession) -> AccountBillingSession {
    AccountBillingSession {
        url: value.checkout_url,
        provider: value.provider,
        expires_at: empty_to_none(value.expires_at),
    }
}

pub fn portal_session_from_grpc(value: PortalSession) -> AccountBillingSession {
    AccountBillingSession {
        url: value.portal_url,
        provider: value.provider,
        expires_at: None,
    }
}

pub fn grpc_error(error: tonic::Status) -> AppError {
    let status = match error.code() {
        Code::InvalidArgument => StatusCode::BAD_REQUEST,
        Code::Unauthenticated => StatusCode::UNAUTHORIZED,
        Code::PermissionDenied => StatusCode::FORBIDDEN,
        Code::NotFound => StatusCode::NOT_FOUND,
        Code::FailedPrecondition => StatusCode::CONFLICT,
        Code::Unavailable | Code::DeadlineExceeded => StatusCode::BAD_GATEWAY,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    AppError::new(status, "billing_grpc_error", error.message().to_string())
}

fn invoice_from_grpc(
    value: crate::grpc_pb::nvbes::billing::v1::BillingInvoice,
) -> AccountBillingInvoice {
    AccountBillingInvoice {
        invoice_id: value.invoice_id,
        invoice_number: empty_to_none(value.invoice_number),
        status: value.status,
        total_minor: value.total_minor,
        currency: value.currency,
        issued_at: empty_to_none(value.issued_at),
        providers: value
            .providers
            .into_iter()
            .map(provider_from_grpc)
            .collect(),
    }
}

fn payment_method_from_grpc(
    value: crate::grpc_pb::nvbes::billing::v1::BillingPaymentMethod,
) -> AccountBillingPaymentMethod {
    AccountBillingPaymentMethod {
        payment_method_id: value.payment_method_id,
        brand: empty_to_none(value.brand),
        last4: empty_to_none(value.last4),
        exp_month: non_zero_i32(value.exp_month),
        exp_year: non_zero_i32(value.exp_year),
        providers: value
            .providers
            .into_iter()
            .map(provider_from_grpc)
            .collect(),
    }
}

fn subscription_from_grpc(
    value: crate::grpc_pb::nvbes::billing::v1::BillingSubscription,
) -> AccountBillingSubscription {
    AccountBillingSubscription {
        subscription_id: value.subscription_id,
        plan_code: empty_to_none(value.plan_code),
        status: value.status,
        providers: value
            .providers
            .into_iter()
            .map(provider_from_grpc)
            .collect(),
    }
}

fn provider_from_grpc(
    value: crate::grpc_pb::nvbes::billing::v1::BillingProviderReference,
) -> AccountBillingProviderReference {
    AccountBillingProviderReference {
        provider: value.provider,
        status: value.status,
        primary: value.primary,
        fallback_eligible: value.fallback_eligible,
    }
}

fn empty_to_none(value: String) -> Option<String> {
    let value = value.trim().to_string();
    if value.is_empty() { None } else { Some(value) }
}

fn non_zero_i32(value: i32) -> Option<i32> {
    if value == 0 { None } else { Some(value) }
}

fn header_value<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers
        .get(name)
        .or_else(|| {
            if name == "x-request-id" {
                headers.get(header::HeaderName::from_static("x-amzn-trace-id"))
            } else {
                None
            }
        })
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::overview_from_grpc;
    use crate::grpc_pb::nvbes::billing::v1::{
        BillingOverview as GrpcBillingOverview, ProductEntitlements,
    };

    #[test]
    fn account_billing_overview_keeps_summary_contract() {
        let overview = overview_from_grpc(GrpcBillingOverview {
            workspace_id: "workspace_123".to_string(),
            plan_code: "team".to_string(),
            subscription_status: "active".to_string(),
            billing_provider: "primary".to_string(),
            current_period_start: "2026-07-01T00:00:00Z".to_string(),
            current_period_end: String::new(),
            entitlements: Some(ProductEntitlements {
                included_storage_gb: 500,
                included_users: 10,
                retention_days: 365,
                max_share_links: 100,
                audit_level: "advanced".to_string(),
            }),
        });

        assert_eq!(overview.workspace_id, "workspace_123");
        assert_eq!(overview.plan_code, "team");
        assert_eq!(overview.subscription_status, "active");
        assert_eq!(overview.current_period_end, None);
        assert_eq!(overview.entitlements.included_storage_gb, 500);
        assert_eq!(overview.entitlements.audit_level, "advanced");
    }
}
