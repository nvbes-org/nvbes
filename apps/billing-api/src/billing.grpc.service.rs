use std::{future::Future, net::SocketAddr};

use chrono::{DateTime, Utc};
use tonic::{Request, Response, Status, transport::Server};
use uuid::Uuid;

use crate::{
    app::BillingAppState,
    grpc::{
        pb::nvbes::{
            billing::v1::{
                BillingOverview, BillingPortal, CheckoutSession, CreateCheckoutRequest,
                GetBillingOverviewRequest, GetBillingPortalRequest, ReconciliationRun,
                RunReconciliationRequest, UsageEvent, UsageIngestResponse,
                billing_service_server::{BillingService, BillingServiceServer},
            },
            platform::v1::RequestContext,
        },
        service_conversions,
    },
};

#[derive(Clone)]
pub struct BillingGrpcService {
    state: BillingAppState,
}

impl BillingGrpcService {
    pub fn new(state: BillingAppState) -> Self {
        Self { state }
    }

    pub fn into_server(self) -> BillingServiceServer<Self> {
        BillingServiceServer::new(self)
    }
}

pub async fn serve(
    addr: SocketAddr,
    state: BillingAppState,
    shutdown: impl Future<Output = ()>,
) -> Result<(), tonic::transport::Error> {
    Server::builder()
        .add_service(BillingGrpcService::new(state).into_server())
        .serve_with_shutdown(addr, shutdown)
        .await
}

#[tonic::async_trait]
impl BillingService for BillingGrpcService {
    async fn get_billing_overview(
        &self,
        request: Request<GetBillingOverviewRequest>,
    ) -> Result<Response<BillingOverview>, Status> {
        let request = request.into_inner();
        let workspace_id = workspace_id(&request.workspace_id)?;
        validate_context(request.context.as_ref(), Some(workspace_id))?;

        let overview = nvbes_billing::fetch_workspace_billing_overview(&self.state.db, workspace_id)
            .await
            .map_err(sql_status)?;
        Ok(Response::new(
            service_conversions::billing_overview_response(overview),
        ))
    }

    async fn get_billing_portal(
        &self,
        request: Request<GetBillingPortalRequest>,
    ) -> Result<Response<BillingPortal>, Status> {
        let request = request.into_inner();
        let workspace_id = workspace_id(&request.workspace_id)?;
        validate_context(request.context.as_ref(), Some(workspace_id))?;

        let portal = nvbes_billing::portal_views::fetch_portal_view(&self.state.db, workspace_id)
            .await
            .map_err(sql_status)?;
        Ok(Response::new(service_conversions::billing_portal_view(
            workspace_id,
            portal,
        )))
    }

    async fn create_checkout(
        &self,
        request: Request<CreateCheckoutRequest>,
    ) -> Result<Response<CheckoutSession>, Status> {
        let request = request.into_inner();
        let workspace_id = workspace_id(&request.workspace_id)?;
        let context = validate_context(request.context.as_ref(), Some(workspace_id))?;
        let actor_principal_id = parse_uuid(&context.actor_principal_id, "actor_principal_id")?;

        let checkout = nvbes_billing::checkout_sessions::create_billing_checkout_session(
            &self.state.db,
            &self.state.config,
            nvbes_billing::checkout_sessions::CreateBillingCheckoutSessionInput {
                workspace_id,
                actor_principal_id,
                checkout: nvbes_billing::types::CreateCheckoutInput {
                    plan_code: request.plan_code,
                    success_url: empty_to_none(request.success_url),
                    cancel_url: empty_to_none(request.cancel_url),
                },
                ip: None,
                trusted_country_header: None,
                user_agent: Some("nvbes-billing-grpc".to_string()),
            },
        )
        .await
        .map_err(checkout_status)?;

        Ok(Response::new(
            service_conversions::checkout_session_response(checkout),
        ))
    }

    async fn ingest_usage(
        &self,
        request: Request<UsageEvent>,
    ) -> Result<Response<UsageIngestResponse>, Status> {
        let request = request.into_inner();
        validate_context(request.context.as_ref(), None)?;
        let tenant_id = parse_uuid(&request.tenant_id, "tenant_id")?;
        let workspace_id = optional_uuid(&request.workspace_id, "workspace_id")?;
        let occurred_at = parse_datetime(&request.occurred_at, "occurred_at")?;

        let response = nvbes_billing::usage::ingest_usage_event(
            &self.state.db,
            nvbes_billing::usage::UsageEvent {
                tenant_id,
                workspace_id,
                meter_code: request.meter_code,
                quantity: request.quantity,
                unit: request.unit,
                occurred_at,
                source: request.source,
                idempotency_key: request.idempotency_key,
            },
        )
        .await
        .map_err(usage_status)?;

        Ok(Response::new(UsageIngestResponse {
            accepted: response.accepted,
        }))
    }

    async fn run_reconciliation(
        &self,
        request: Request<RunReconciliationRequest>,
    ) -> Result<Response<ReconciliationRun>, Status> {
        let request = request.into_inner();
        validate_context(request.context.as_ref(), None)?;
        let period_end = if request.period_end.trim().is_empty() {
            Utc::now()
        } else {
            parse_datetime(&request.period_end, "period_end")?
        };

        let result = nvbes_billing::reconciliation_db::run_ledger_reconciliation(
            &self.state.db,
            period_end,
        )
        .await
        .map_err(reconciliation_status)?;

        Ok(Response::new(service_conversions::reconciliation_run(
            result,
        )))
    }
}

fn validate_context(
    context: Option<&RequestContext>,
    workspace_id: Option<Uuid>,
) -> Result<&RequestContext, Status> {
    let context = context.ok_or_else(|| {
        Status::invalid_argument("request context is required for Billing gRPC calls")
    })?;
    if context.request_id.trim().is_empty() || context.actor_principal_id.trim().is_empty() {
        return Err(Status::invalid_argument(
            "request_id and actor_principal_id are required in request context",
        ));
    }
    if let Some(workspace_id) = workspace_id {
        let tenant = context.tenant.as_ref().ok_or_else(|| {
            Status::invalid_argument("tenant context is required for workspace Billing calls")
        })?;
        if tenant.workspace_id != workspace_id.to_string() {
            return Err(Status::permission_denied(
                "request context workspace does not match the Billing workspace",
            ));
        }
    }
    Ok(context)
}

fn workspace_id(value: &str) -> Result<Uuid, Status> {
    parse_uuid(value, "workspace_id")
}

fn parse_uuid(value: &str, field: &'static str) -> Result<Uuid, Status> {
    Uuid::parse_str(value)
        .map_err(|_| Status::invalid_argument(format!("{field} must be a valid UUID")))
}

fn optional_uuid(value: &str, field: &'static str) -> Result<Option<Uuid>, Status> {
    if value.trim().is_empty() {
        Ok(None)
    } else {
        parse_uuid(value, field).map(Some)
    }
}

fn parse_datetime(value: &str, field: &'static str) -> Result<DateTime<Utc>, Status> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|_| Status::invalid_argument(format!("{field} must be an RFC3339 timestamp")))
}

fn empty_to_none(value: String) -> Option<String> {
    let value = value.trim().to_string();
    if value.is_empty() { None } else { Some(value) }
}

fn sql_status(error: sqlx::Error) -> Status {
    match error {
        sqlx::Error::RowNotFound => Status::not_found("billing workspace was not found"),
        error => Status::internal(format!("billing database error: {error}")),
    }
}

fn checkout_status(error: nvbes_billing::checkout_sessions::BillingCheckoutSessionError) -> Status {
    use nvbes_billing::checkout_sessions::BillingCheckoutSessionError;

    match error {
        BillingCheckoutSessionError::WorkspaceNotFound => {
            Status::not_found("billing workspace was not found")
        }
        BillingCheckoutSessionError::InvalidRedirect { field, .. } => {
            Status::invalid_argument(format!("{field} is not an allowed redirect URL"))
        }
        BillingCheckoutSessionError::BillingLocked
        | BillingCheckoutSessionError::ManualReviewHold
        | BillingCheckoutSessionError::FraudBlocked
        | BillingCheckoutSessionError::RoutingBlocked(_) => {
            Status::failed_precondition(error.to_string())
        }
        BillingCheckoutSessionError::FraudPolicy(_)
        | BillingCheckoutSessionError::RoutingRule(_)
        | BillingCheckoutSessionError::CheckoutProvider(_)
        | BillingCheckoutSessionError::Database(_) => Status::internal(error.to_string()),
    }
}

fn usage_status(error: nvbes_billing::usage::BillingUsageIngestError) -> Status {
    match error {
        nvbes_billing::usage::BillingUsageIngestError::Validation { code, message } => {
            Status::invalid_argument(format!("{code}: {message}"))
        }
        nvbes_billing::usage::BillingUsageIngestError::Database(_)
        | nvbes_billing::usage::BillingUsageIngestError::Serialization(_) => {
            Status::internal(error.to_string())
        }
    }
}

fn reconciliation_status(
    error: nvbes_billing::reconciliation_db::BillingReconciliationError,
) -> Status {
    Status::internal(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::validate_context;
    use crate::grpc::pb::nvbes::platform::v1::{RequestContext, TenantContext};
    use uuid::Uuid;

    #[test]
    fn context_workspace_must_match_request_workspace() {
        let workspace_id = Uuid::new_v4();
        let context = RequestContext {
            request_id: "req_123".to_string(),
            correlation_id: "corr_123".to_string(),
            actor_principal_id: Uuid::new_v4().to_string(),
            tenant: Some(TenantContext {
                tenant_id: Uuid::new_v4().to_string(),
                workspace_id: workspace_id.to_string(),
                region_id: "eu".to_string(),
                data_residency: "eu".to_string(),
            }),
        };

        assert!(validate_context(Some(&context), Some(workspace_id)).is_ok());
        assert!(
            validate_context(Some(&context), Some(Uuid::new_v4()))
                .expect_err("mismatched workspace should be rejected")
                .message()
                .contains("workspace")
        );
    }
}
