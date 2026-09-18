use tonic::{Request, Response, Status};
use uuid::Uuid;

use nvbes_billing::{
    fetch_workspace_billing_overview,
    proto::nvbes::billing::v1::{
        BillingEventReceipt, BillingOperationsSnapshot, CreateBillingPortalSessionRequest,
        CreateBillingPortalSessionResponse, CreateCheckoutSessionRequest,
        CreateCheckoutSessionResponse, GetBillingOperationsSnapshotRequest,
        GetWorkspaceBillingOverviewRequest, GetWorkspaceEntitlementsRequest,
        ReconcileWorkspaceRequest, ReconcileWorkspaceResponse, SubmitBillingEventRequest,
        WorkspaceBillingOverviewResponse, WorkspaceEntitlementsResponse,
        billing_delivery_service_server::{BillingDeliveryService, BillingDeliveryServiceServer},
        billing_operations_service_server::{
            BillingOperationsService, BillingOperationsServiceServer,
        },
    },
};
use prost_types::Timestamp;

use crate::{customer::get_or_create_customer, outbox::record_outbox_event};

pub const MAX_GRPC_DECODE_BYTES: usize = 256 * 1024;

// ── Server constructors ───────────────────────────────────────────────────────

#[derive(Clone)]
pub struct BillingDeliveryGrpcService {
    state: crate::app::BillingState,
}

impl BillingDeliveryGrpcService {
    pub fn new(state: crate::app::BillingState) -> Self {
        Self { state }
    }
}

pub fn delivery_server(
    service: BillingDeliveryGrpcService,
) -> BillingDeliveryServiceServer<BillingDeliveryGrpcService> {
    BillingDeliveryServiceServer::new(service).max_decoding_message_size(MAX_GRPC_DECODE_BYTES)
}

#[derive(Clone)]
pub struct BillingOperationsGrpcService {
    state: crate::app::BillingState,
}

impl BillingOperationsGrpcService {
    pub fn new(state: crate::app::BillingState) -> Self {
        Self { state }
    }
}

pub fn operations_server(
    service: BillingOperationsGrpcService,
) -> BillingOperationsServiceServer<BillingOperationsGrpcService> {
    BillingOperationsServiceServer::new(service).max_decoding_message_size(MAX_GRPC_DECODE_BYTES)
}

// ── Auth helper ───────────────────────────────────────────────────────────────

fn require_grpc_token<T>(
    req: &Request<T>,
    config: &crate::config::BillingConfig,
) -> Result<(), Status> {
    let token = req
        .metadata()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .ok_or_else(|| Status::unauthenticated("missing authorization"))?;

    let expected = config
        .operator_token
        .as_deref()
        .ok_or_else(|| Status::unauthenticated("internal auth not configured"))?;

    if nvbes_billing::stripe::constant_time_eq(token.as_bytes(), expected.as_bytes()) {
        Ok(())
    } else {
        Err(Status::unauthenticated("invalid token"))
    }
}

// ── BillingDeliveryService ────────────────────────────────────────────────────

#[tonic::async_trait]
impl BillingDeliveryService for BillingDeliveryGrpcService {
    #[tracing::instrument(
        name = "billing.get_workspace_entitlements",
        skip_all,
        fields(rpc.system = "grpc", workspace_id = tracing::field::Empty)
    )]
    async fn get_workspace_entitlements(
        &self,
        request: Request<GetWorkspaceEntitlementsRequest>,
    ) -> Result<Response<WorkspaceEntitlementsResponse>, Status> {
        require_grpc_token(&request, &self.state.config)?;
        let workspace_id = parse_uuid(&request.get_ref().workspace_id)?;
        tracing::Span::current().record("workspace_id", workspace_id.to_string());

        let overview = fetch_workspace_billing_overview(&self.state.db, workspace_id)
            .await
            .map_err(|e| {
                tracing::error!(error = ?e, "get_workspace_entitlements db error");
                Status::internal("entitlements unavailable")
            })?;

        let ent = &overview.entitlements;
        let sub = &overview.subscription;
        let plan = &overview.plan;

        Ok(Response::new(WorkspaceEntitlementsResponse {
            workspace_id: workspace_id.to_string(),
            plan_code: plan.code.clone(),
            status: sub.status.clone(),
            max_seats: i64::from(plan.included_users),
            storage_bytes_quota: i64::from(plan.included_storage_gb) * 1024 * 1024 * 1024,
            api_rate_limit_per_minute: i64::from(ent.api_key_limit) * 60,
            custom_domain_enabled: false,
            advanced_security_enabled: false,
            current_period_end: sub.current_period_end.map(chrono_to_proto_ts),
        }))
    }

    #[tracing::instrument(
        name = "billing.get_workspace_billing_overview",
        skip_all,
        fields(rpc.system = "grpc", workspace_id = tracing::field::Empty)
    )]
    async fn get_workspace_billing_overview(
        &self,
        request: Request<GetWorkspaceBillingOverviewRequest>,
    ) -> Result<Response<WorkspaceBillingOverviewResponse>, Status> {
        require_grpc_token(&request, &self.state.config)?;
        let workspace_id = parse_uuid(&request.get_ref().workspace_id)?;
        tracing::Span::current().record("workspace_id", workspace_id.to_string());

        let overview = fetch_workspace_billing_overview(&self.state.db, workspace_id)
            .await
            .map_err(|e| {
                tracing::error!(error = ?e, "get_workspace_billing_overview db error");
                Status::internal("billing overview unavailable")
            })?;

        let sub = &overview.subscription;
        let plan = &overview.plan;

        Ok(Response::new(WorkspaceBillingOverviewResponse {
            workspace_id: workspace_id.to_string(),
            plan_code: plan.code.clone(),
            status: sub.status.clone(),
            customer_id: overview
                .billing_account
                .billing_email
                .clone()
                .unwrap_or_default(),
            monthly_price_cents: plan.monthly_price_cents,
            currency: plan.currency.clone(),
            active_seats: overview.invoice_estimate.seat_overage_amount_cents,
            storage_bytes_used: i64::from(plan.included_storage_gb) * 1024 * 1024 * 1024,
            cancel_at_period_end: None,
            current_period_end: sub.current_period_end.map(chrono_to_proto_ts),
        }))
    }

    #[tracing::instrument(
        name = "billing.create_checkout_session",
        skip_all,
        fields(rpc.system = "grpc", workspace_id = tracing::field::Empty)
    )]
    async fn create_checkout_session(
        &self,
        request: Request<CreateCheckoutSessionRequest>,
    ) -> Result<Response<CreateCheckoutSessionResponse>, Status> {
        require_grpc_token(&request, &self.state.config)?;
        let req = request.into_inner();
        let workspace_id = parse_uuid(&req.workspace_id)?;
        tracing::Span::current().record("workspace_id", workspace_id.to_string());

        if req.plan_code.is_empty() || req.success_url.is_empty() {
            return Err(Status::invalid_argument(
                "plan_code and success_url are required",
            ));
        }

        let customer_id = get_or_create_customer(
            &self.state.db,
            &self.state.config,
            workspace_id,
            "team",
            None,
        )
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, "create_checkout_session customer error");
            Status::internal("could not resolve customer")
        })?;

        let (checkout_url, session_id) = if self
            .state
            .config
            .stripe_secret_key
            .starts_with("sk_test_dummy")
        {
            let sid = format!("cs_test_{}", Uuid::new_v4().simple());
            let url = format!(
                "{}/billing/mock-checkout?session_id={sid}",
                self.state.config.app_url
            );
            (url, sid)
        } else {
            crate::checkout::create_stripe_checkout_grpc(
                &self.state,
                &customer_id,
                workspace_id,
                &req.plan_code,
                &req.idempotency_key,
            )
            .await
            .map_err(|e| {
                tracing::error!(error = ?e, "create_checkout_session stripe error");
                Status::internal("checkout session could not be created")
            })?
        };

        Ok(Response::new(CreateCheckoutSessionResponse {
            session_id,
            checkout_url,
            status: "open".to_string(),
        }))
    }

    #[tracing::instrument(
        name = "billing.create_billing_portal_session",
        skip_all,
        fields(rpc.system = "grpc", workspace_id = tracing::field::Empty)
    )]
    async fn create_billing_portal_session(
        &self,
        request: Request<CreateBillingPortalSessionRequest>,
    ) -> Result<Response<CreateBillingPortalSessionResponse>, Status> {
        require_grpc_token(&request, &self.state.config)?;
        let req = request.into_inner();
        let workspace_id = parse_uuid(&req.workspace_id)?;
        tracing::Span::current().record("workspace_id", workspace_id.to_string());

        let customer_id = get_or_create_customer(
            &self.state.db,
            &self.state.config,
            workspace_id,
            "team",
            None,
        )
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, "create_billing_portal_session customer error");
            Status::internal("could not resolve customer")
        })?;

        let portal_url = if self
            .state
            .config
            .stripe_secret_key
            .starts_with("sk_test_dummy")
        {
            format!(
                "{}/billing/mock-portal?customer={customer_id}",
                self.state.config.app_url
            )
        } else {
            crate::portal::create_stripe_portal_grpc(&self.state, &customer_id)
                .await
                .map_err(|e| {
                    tracing::error!(error = ?e, "create_billing_portal_session stripe error");
                    Status::internal("portal session could not be created")
                })?
        };

        Ok(Response::new(CreateBillingPortalSessionResponse {
            portal_url,
        }))
    }

    #[tracing::instrument(
        name = "billing.submit_billing_event",
        skip_all,
        fields(rpc.system = "grpc", event_type = tracing::field::Empty)
    )]
    async fn submit_billing_event(
        &self,
        request: Request<SubmitBillingEventRequest>,
    ) -> Result<Response<BillingEventReceipt>, Status> {
        require_grpc_token(&request, &self.state.config)?;
        let req = request.into_inner();
        let aggregate_id = parse_uuid(&req.aggregate_id)?;
        tracing::Span::current().record("event_type", &req.event_type);

        if req.event_type.is_empty() || req.payload_json.is_empty() {
            return Err(Status::invalid_argument(
                "event_type and payload_json are required",
            ));
        }
        let payload: serde_json::Value = serde_json::from_str(&req.payload_json)
            .map_err(|_| Status::invalid_argument("payload_json is not valid JSON"))?;

        record_outbox_event(&self.state.db, &req.event_type, aggregate_id, &payload)
            .await
            .map_err(|e| {
                tracing::error!(error = ?e, "submit_billing_event outbox error");
                Status::internal("event could not be durably accepted")
            })?;

        let now = chrono::Utc::now();
        Ok(Response::new(BillingEventReceipt {
            event_id: Uuid::new_v4().to_string(),
            accepted_at: Some(chrono_to_proto_ts(now)),
            duplicate: false,
        }))
    }
}

// ── BillingOperationsService ──────────────────────────────────────────────────

#[tonic::async_trait]
impl BillingOperationsService for BillingOperationsGrpcService {
    #[tracing::instrument(
        name = "billing.get_billing_operations_snapshot",
        skip_all,
        fields(rpc.system = "grpc")
    )]
    async fn get_billing_operations_snapshot(
        &self,
        request: Request<GetBillingOperationsSnapshotRequest>,
    ) -> Result<Response<BillingOperationsSnapshot>, Status> {
        require_grpc_token(&request, &self.state.config)?;

        let active: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM billing_subscriptions WHERE status = 'active'",
        )
        .fetch_one(&self.state.db)
        .await
        .map_err(|_| Status::internal("db error"))?;

        let past_due: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM billing_subscriptions WHERE status = 'past_due'",
        )
        .fetch_one(&self.state.db)
        .await
        .map_err(|_| Status::internal("db error"))?;

        let pending_outbox: i64 =
            sqlx::query_scalar("SELECT count(*) FROM billing_outbox WHERE published_at IS NULL")
                .fetch_one(&self.state.db)
                .await
                .map_err(|_| Status::internal("db error"))?;

        let unresolved: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM billing_reconciliation_items WHERE status = 'pending'",
        )
        .fetch_one(&self.state.db)
        .await
        .map_err(|_| Status::internal("db error"))?;

        let mrr: i64 = sqlx::query_scalar(
            "SELECT coalesce(sum(monthly_price_cents), 0) \
             FROM billing_subscriptions WHERE status = 'active'",
        )
        .fetch_one(&self.state.db)
        .await
        .map_err(|_| Status::internal("db error"))?;

        Ok(Response::new(BillingOperationsSnapshot {
            active_subscriptions_count: active,
            past_due_subscriptions_count: past_due,
            pending_outbox_count: pending_outbox,
            unresolved_reconciliation_count: unresolved,
            monthly_recurring_revenue_cents: mrr,
            snapshot_at: Some(chrono_to_proto_ts(chrono::Utc::now())),
        }))
    }

    #[tracing::instrument(
        name = "billing.reconcile_workspace",
        skip_all,
        fields(rpc.system = "grpc", workspace_id = tracing::field::Empty)
    )]
    async fn reconcile_workspace(
        &self,
        request: Request<ReconcileWorkspaceRequest>,
    ) -> Result<Response<ReconcileWorkspaceResponse>, Status> {
        require_grpc_token(&request, &self.state.config)?;
        let workspace_id = parse_uuid(&request.get_ref().workspace_id)?;
        tracing::Span::current().record("workspace_id", workspace_id.to_string());

        let status: Option<String> = sqlx::query_scalar(
            "SELECT status FROM billing_subscriptions WHERE account_id = $1 \
             ORDER BY updated_at DESC LIMIT 1",
        )
        .bind(workspace_id)
        .fetch_optional(&self.state.db)
        .await
        .map_err(|_| Status::internal("db error"))?;

        Ok(Response::new(ReconcileWorkspaceResponse {
            workspace_id: workspace_id.to_string(),
            status_synced: true,
            current_status: status.unwrap_or_else(|| "none".to_string()),
        }))
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn parse_uuid(s: &str) -> Result<Uuid, Status> {
    Uuid::parse_str(s).map_err(|_| Status::invalid_argument("invalid workspace_id UUID"))
}

fn chrono_to_proto_ts(dt: chrono::DateTime<chrono::Utc>) -> Timestamp {
    Timestamp {
        seconds: dt.timestamp(),
        nanos: dt.timestamp_subsec_nanos() as i32,
    }
}
