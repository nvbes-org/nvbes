use std::{future::Future, net::SocketAddr};

use chrono::Utc;
use tonic::{Request, Response, Status, transport::Server};
use uuid::Uuid;

use crate::{
    app::BillingAppState,
    grpc::{
        pb::nvbes::{
            billing::v1 as billing,
            billing::v1::billing_service_server::{BillingService, BillingServiceServer},
            platform::v1::RequestContext,
        },
        service_admin, service_admin_billing, service_admin_entitlements, service_admin_exports,
        service_admin_operations, service_admin_platform,
        service_admin_platform_routing_simulation, service_admin_platform_snapshot,
        service_admin_revenue, service_admin_revenue_snapshot, service_admin_risk,
        service_admin_usage, service_conversions,
        service_status::{
            checkout_status, empty_to_none, optional_uuid, parse_datetime, parse_uuid,
            portal_status, reconciliation_status, sql_status, usage_status, validate_context,
            workspace_id,
        },
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
        request: Request<billing::GetBillingOverviewRequest>,
    ) -> Result<Response<billing::BillingOverview>, Status> {
        let request = request.into_inner();
        let workspace_id = workspace_id(&request.workspace_id)?;
        validate_context(request.context.as_ref(), Some(workspace_id))?;
        let overview =
            nvbes_billing::fetch_workspace_billing_overview(&self.state.db, workspace_id)
                .await
                .map_err(sql_status)?;
        Ok(Response::new(
            service_conversions::billing_overview_response(overview),
        ))
    }

    async fn get_billing_portal(
        &self,
        request: Request<billing::GetBillingPortalRequest>,
    ) -> Result<Response<billing::BillingPortal>, Status> {
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
        request: Request<billing::CreateCheckoutRequest>,
    ) -> Result<Response<billing::CheckoutSession>, Status> {
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

    async fn create_portal(
        &self,
        request: Request<billing::CreatePortalRequest>,
    ) -> Result<Response<billing::PortalSession>, Status> {
        let request = request.into_inner();
        let workspace_id = workspace_id(&request.workspace_id)?;
        let context = validate_context(request.context.as_ref(), Some(workspace_id))?;
        let actor_principal_id = parse_uuid(&context.actor_principal_id, "actor_principal_id")?;
        let portal = nvbes_billing::portal_actions::create_billing_portal_session(
            &self.state.db,
            &self.state.config,
            nvbes_billing::portal_actions::CreateBillingPortalSessionInput {
                workspace_id,
                actor_principal_id,
                portal: nvbes_billing::types::CreatePortalInput {
                    return_url: empty_to_none(request.return_url),
                },
                ip: None,
                user_agent: Some("nvbes-billing-grpc".to_string()),
            },
        )
        .await
        .map_err(portal_status)?;
        Ok(Response::new(service_conversions::portal_session_response(
            portal,
        )))
    }

    async fn get_entitlements(
        &self,
        request: Request<billing::GetEntitlementsRequest>,
    ) -> Result<Response<billing::EntitlementSnapshot>, Status> {
        let request = request.into_inner();
        let workspace_id = workspace_id(&request.workspace_id)?;
        validate_context(request.context.as_ref(), Some(workspace_id))?;
        let overview =
            nvbes_billing::fetch_workspace_billing_overview(&self.state.db, workspace_id)
                .await
                .map_err(sql_status)?;
        Ok(Response::new(service_conversions::entitlement_snapshot(
            overview,
        )))
    }

    async fn list_invoices(
        &self,
        request: Request<billing::ListInvoicesRequest>,
    ) -> Result<Response<billing::ListInvoicesResponse>, Status> {
        let request = request.into_inner();
        let workspace_id = workspace_id(&request.workspace_id)?;
        validate_context(request.context.as_ref(), Some(workspace_id))?;
        let invoices =
            nvbes_billing::portal_views::fetch_portal_invoices(&self.state.db, workspace_id)
                .await
                .map_err(sql_status)?
                .into_iter()
                .map(service_conversions::billing_invoice)
                .collect();
        Ok(Response::new(billing::ListInvoicesResponse {
            invoices,
            page: None,
        }))
    }

    async fn get_invoice(
        &self,
        request: Request<billing::GetInvoiceRequest>,
    ) -> Result<Response<billing::BillingInvoice>, Status> {
        let request = request.into_inner();
        let workspace_id = workspace_id(&request.workspace_id)?;
        validate_context(request.context.as_ref(), Some(workspace_id))?;
        let invoice_id = parse_uuid(&request.invoice_id, "invoice_id")?.to_string();
        let invoice =
            nvbes_billing::portal_views::fetch_portal_invoices(&self.state.db, workspace_id)
                .await
                .map_err(sql_status)?
                .into_iter()
                .map(service_conversions::billing_invoice)
                .find(|invoice| invoice.invoice_id == invoice_id)
                .ok_or_else(|| Status::not_found("billing invoice was not found"))?;
        Ok(Response::new(invoice))
    }

    async fn record_ledger_entry(
        &self,
        _request: Request<billing::RecordLedgerEntryRequest>,
    ) -> Result<Response<billing::LedgerEntry>, Status> {
        Err(Status::unimplemented(
            "Billing ledger write RPC is not implemented by billing-service yet",
        ))
    }

    async fn list_ledger_entries(
        &self,
        _request: Request<billing::ListLedgerEntriesRequest>,
    ) -> Result<Response<billing::ListLedgerEntriesResponse>, Status> {
        Err(Status::unimplemented(
            "Billing ledger query RPC is not implemented by billing-service yet",
        ))
    }

    async fn ingest_provider_webhook(
        &self,
        _request: Request<billing::IngestProviderWebhookRequest>,
    ) -> Result<Response<billing::ProviderWebhookEvent>, Status> {
        Err(Status::unimplemented(
            "Billing provider webhook RPC is not implemented by billing-service yet",
        ))
    }

    async fn list_provider_webhook_events(
        &self,
        _request: Request<billing::ListProviderWebhookEventsRequest>,
    ) -> Result<Response<billing::ListProviderWebhookEventsResponse>, Status> {
        Err(Status::unimplemented(
            "Billing provider webhook listing RPC is not implemented by billing-service yet",
        ))
    }

    async fn reconcile_provider_webhook(
        &self,
        _request: Request<billing::ReconcileProviderWebhookRequest>,
    ) -> Result<Response<billing::ProviderWebhookEvent>, Status> {
        Err(Status::unimplemented(
            "Billing provider webhook reconciliation RPC is not implemented by billing-service yet",
        ))
    }

    async fn get_admin_command_center_billing_metrics(
        &self,
        request: Request<billing::GetAdminCommandCenterBillingMetricsRequest>,
    ) -> Result<Response<billing::AdminCommandCenterBillingMetrics>, Status> {
        let request = request.into_inner();
        validate_context(request.context.as_ref(), None)?;
        Ok(Response::new(
            service_admin::command_center_metrics(&self.state.db).await?,
        ))
    }

    async fn get_admin_operations_center(
        &self,
        request: Request<billing::GetAdminOperationsCenterRequest>,
    ) -> Result<Response<billing::AdminOperationsCenterSnapshot>, Status> {
        let request = request.into_inner();
        validate_context(request.context.as_ref(), None)?;
        Ok(Response::new(
            service_admin::operations_center_snapshot(&self.state.db).await?,
        ))
    }

    async fn get_admin_billing_overview(
        &self,
        request: Request<billing::GetAdminBillingOverviewRequest>,
    ) -> Result<Response<billing::AdminBillingOverview>, Status> {
        let request = request.into_inner();
        let workspace_id = workspace_id(&request.workspace_id)?;
        let context = validate_context(request.context.as_ref(), Some(workspace_id))?;
        Ok(Response::new(
            service_admin::billing_overview(&self.state.db, tenant_id(context)?).await?,
        ))
    }

    async fn list_admin_provider_event_failures(
        &self,
        request: Request<billing::ListAdminProviderEventFailuresRequest>,
    ) -> Result<Response<billing::AdminProviderEventFailures>, Status> {
        let request = request.into_inner();
        let workspace_id = workspace_id(&request.workspace_id)?;
        let context = validate_context(request.context.as_ref(), Some(workspace_id))?;
        Ok(Response::new(
            service_admin::provider_event_failures(
                &self.state.db,
                tenant_id(context)?,
                i64::from(request.limit),
            )
            .await?,
        ))
    }

    async fn search_admin_billing(
        &self,
        request: Request<billing::SearchAdminBillingRequest>,
    ) -> Result<Response<billing::AdminBillingSearchResults>, Status> {
        let request = request.into_inner();
        let workspace_id = workspace_id(&request.workspace_id)?;
        let context = validate_context(request.context.as_ref(), Some(workspace_id))?;
        Ok(Response::new(
            service_admin::search_billing(
                &self.state.db,
                tenant_id(context)?,
                &request.query,
                i64::from(request.limit),
            )
            .await?,
        ))
    }

    async fn build_admin_finance_export(
        &self,
        request: Request<billing::BuildAdminFinanceExportRequest>,
    ) -> Result<Response<billing::AdminFinanceExport>, Status> {
        let request = request.into_inner();
        let workspace_id = workspace_id(&request.workspace_id)?;
        let context = validate_context(request.context.as_ref(), Some(workspace_id))?;
        Ok(Response::new(
            service_admin_exports::build_finance_export(
                &self.state.db,
                tenant_id(context)?,
                &request.export_type,
            )
            .await?,
        ))
    }

    async fn get_admin_usage_center(
        &self,
        request: Request<billing::GetAdminUsageCenterRequest>,
    ) -> Result<Response<billing::AdminUsageCenterSnapshot>, Status> {
        let request = request.into_inner();
        validate_context(request.context.as_ref(), None)?;
        Ok(Response::new(
            service_admin_usage::usage_center(&self.state.db).await?,
        ))
    }

    async fn run_admin_usage_action(
        &self,
        request: Request<billing::AdminUsageActionRequest>,
    ) -> Result<Response<billing::AdminUsageActionResult>, Status> {
        let request = request.into_inner();
        let workspace_id = workspace_id(&request.workspace_id)?;
        let context = validate_context(request.context.as_ref(), Some(workspace_id))?;
        Ok(Response::new(
            service_admin_usage::run_usage_action(
                &self.state.db,
                tenant_id(context)?,
                actor_principal_id(context)?,
                request,
            )
            .await?,
        ))
    }

    async fn get_admin_entitlements_center(
        &self,
        request: Request<billing::GetAdminEntitlementsCenterRequest>,
    ) -> Result<Response<billing::AdminEntitlementsCenterSnapshot>, Status> {
        let request = request.into_inner();
        validate_context(request.context.as_ref(), None)?;
        Ok(Response::new(
            service_admin_entitlements::entitlements_center(&self.state.db).await?,
        ))
    }

    async fn get_admin_billing_platform_center(
        &self,
        request: Request<billing::GetAdminBillingPlatformCenterRequest>,
    ) -> Result<Response<billing::AdminBillingPlatformCenterSnapshot>, Status> {
        let request = request.into_inner();
        validate_context(request.context.as_ref(), None)?;
        Ok(Response::new(
            service_admin_platform_snapshot::billing_platform_center(&self.state.db).await?,
        ))
    }

    async fn simulate_admin_billing_routing(
        &self,
        request: Request<billing::SimulateAdminBillingRoutingRequest>,
    ) -> Result<Response<billing::AdminBillingRoutingSimulationResult>, Status> {
        let request = request.into_inner();
        let workspace_id = workspace_id(&request.workspace_id)?;
        validate_context(request.context.as_ref(), Some(workspace_id))?;
        Ok(Response::new(
            service_admin_platform_routing_simulation::simulate_routing(&self.state.db, request)
                .await?,
        ))
    }

    async fn run_admin_operations_action(
        &self,
        request: Request<billing::AdminOperationsActionRequest>,
    ) -> Result<Response<billing::AdminOperationsActionResult>, Status> {
        let request = request.into_inner();
        let context = validate_context(request.context.as_ref(), None)?;
        let result = service_admin_operations::run_operations_action(
            &self.state.db,
            billing::AdminOperationsActionKind::try_from(request.action_kind)
                .unwrap_or(billing::AdminOperationsActionKind::Unspecified),
            tenant_id(context)?,
            actor_principal_id(context)?,
            parse_uuid(&request.target_id, "target_id")?,
            request.reason,
        )
        .await?;
        Ok(Response::new(result))
    }

    async fn run_admin_billing_platform_action(
        &self,
        request: Request<billing::AdminBillingPlatformActionRequest>,
    ) -> Result<Response<billing::AdminBillingPlatformActionResult>, Status> {
        let request = request.into_inner();
        let context = validate_context(request.context.as_ref(), None)?;
        let result = service_admin_platform::run_platform_action(
            &self.state.db,
            billing::AdminBillingPlatformActionKind::try_from(request.action_kind)
                .unwrap_or(billing::AdminBillingPlatformActionKind::Unspecified),
            tenant_id(context)?,
            actor_principal_id(context)?,
            optional_uuid(&request.target_id, "target_id")?,
            request.reason,
            request.routing_rule,
        )
        .await?;
        Ok(Response::new(result))
    }

    async fn get_admin_revenue_center(
        &self,
        request: Request<billing::GetAdminRevenueCenterRequest>,
    ) -> Result<Response<billing::AdminRevenueCenterSnapshot>, Status> {
        let request = request.into_inner();
        validate_context(request.context.as_ref(), None)?;
        Ok(Response::new(
            service_admin_revenue_snapshot::revenue_center(&self.state.db).await?,
        ))
    }

    async fn run_admin_billing_action(
        &self,
        request: Request<billing::AdminBillingActionRequest>,
    ) -> Result<Response<billing::AdminBillingActionResult>, Status> {
        let request = request.into_inner();
        let context = validate_context(request.context.as_ref(), None)?;
        let action_kind = billing::AdminBillingActionKind::try_from(request.action_kind)
            .unwrap_or(billing::AdminBillingActionKind::Unspecified);
        let result = service_admin_billing::run_billing_admin_action(
            &self.state.db,
            tenant_id(context)?,
            actor_principal_id(context)?,
            action_kind,
            request,
        )
        .await?;
        Ok(Response::new(result))
    }

    async fn get_admin_risk_decision_center(
        &self,
        request: Request<billing::GetAdminRiskDecisionCenterRequest>,
    ) -> Result<Response<billing::AdminRiskDecisionCenterSnapshot>, Status> {
        let request = request.into_inner();
        validate_context(request.context.as_ref(), None)?;
        Ok(Response::new(
            service_admin_risk::risk_decision_center(&self.state.db).await?,
        ))
    }

    async fn run_admin_risk_action(
        &self,
        request: Request<billing::AdminRiskActionRequest>,
    ) -> Result<Response<billing::AdminRiskActionResult>, Status> {
        let request = request.into_inner();
        let workspace_id = workspace_id(&request.workspace_id)?;
        let context = validate_context(request.context.as_ref(), Some(workspace_id))?;
        let result = service_admin_risk::run_risk_action(
            &self.state.db,
            billing::AdminRiskActionKind::try_from(request.action_kind)
                .unwrap_or(billing::AdminRiskActionKind::Unspecified),
            tenant_id(context)?,
            actor_principal_id(context)?,
            workspace_id,
            parse_uuid(&request.target_id, "target_id")?,
            request.reason,
        )
        .await?;
        Ok(Response::new(result))
    }

    async fn run_admin_revenue_action(
        &self,
        request: Request<billing::AdminRevenueActionRequest>,
    ) -> Result<Response<billing::AdminRevenueActionResult>, Status> {
        let request = request.into_inner();
        let context = validate_context(request.context.as_ref(), None)?;
        let result = service_admin_revenue::run_revenue_action(
            &self.state.db,
            billing::AdminRevenueActionKind::try_from(request.action_kind)
                .unwrap_or(billing::AdminRevenueActionKind::Unspecified),
            tenant_id(context)?,
            actor_principal_id(context)?,
            parse_uuid(&request.target_id, "target_id")?,
            request.reason,
        )
        .await?;
        Ok(Response::new(result))
    }

    async fn ingest_usage(
        &self,
        request: Request<billing::UsageEvent>,
    ) -> Result<Response<billing::UsageIngestResponse>, Status> {
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
        Ok(Response::new(billing::UsageIngestResponse {
            accepted: response.accepted,
        }))
    }

    async fn run_reconciliation(
        &self,
        request: Request<billing::RunReconciliationRequest>,
    ) -> Result<Response<billing::ReconciliationRun>, Status> {
        let request = request.into_inner();
        validate_context(request.context.as_ref(), None)?;
        let period_end = if request.period_end.trim().is_empty() {
            Utc::now()
        } else {
            parse_datetime(&request.period_end, "period_end")?
        };
        let result =
            nvbes_billing::reconciliation_db::run_ledger_reconciliation(&self.state.db, period_end)
                .await
                .map_err(reconciliation_status)?;
        Ok(Response::new(service_conversions::reconciliation_run(
            result,
        )))
    }
}

fn tenant_id(context: &RequestContext) -> Result<Uuid, Status> {
    let tenant = context
        .tenant
        .as_ref()
        .ok_or_else(|| Status::invalid_argument("tenant context is required"))?;
    parse_uuid(&tenant.tenant_id, "tenant_id")
}

fn actor_principal_id(context: &RequestContext) -> Result<Uuid, Status> {
    parse_uuid(&context.actor_principal_id, "actor_principal_id")
}
