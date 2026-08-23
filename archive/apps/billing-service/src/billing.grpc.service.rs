use chrono::Utc;
use tonic::{Request, Response, Status};
use uuid::Uuid;

use crate::grpc::{
    pb::nvbes::{
        billing::v1 as billing, billing::v1::billing_service_server::BillingService,
        platform::v1::RequestContext,
    },
    service_admin, service_admin_billing, service_admin_entitlements, service_admin_exports,
    service_admin_operations, service_admin_platform, service_admin_platform_routing_simulation,
    service_admin_platform_snapshot, service_admin_revenue, service_admin_revenue_snapshot,
    service_admin_risk, service_admin_usage, service_conversions,
    service_runtime::BillingGrpcService,
    service_status::{
        optional_uuid, parse_datetime, parse_uuid, reconciliation_status, usage_status,
        validate_context, workspace_id,
    },
    service_workspace,
};

pub use super::service_runtime::serve;

#[tonic::async_trait]
impl BillingService for BillingGrpcService {
    async fn get_billing_overview(
        &self,
        request: Request<billing::GetBillingOverviewRequest>,
    ) -> Result<Response<billing::BillingOverview>, Status> {
        service_workspace::get_billing_overview(&self.state, request).await
    }

    async fn get_billing_portal(
        &self,
        request: Request<billing::GetBillingPortalRequest>,
    ) -> Result<Response<billing::BillingPortal>, Status> {
        service_workspace::get_billing_portal(&self.state, request).await
    }

    async fn create_checkout(
        &self,
        request: Request<billing::CreateCheckoutRequest>,
    ) -> Result<Response<billing::CheckoutSession>, Status> {
        service_workspace::create_checkout(&self.state, request).await
    }

    async fn create_portal(
        &self,
        request: Request<billing::CreatePortalRequest>,
    ) -> Result<Response<billing::PortalSession>, Status> {
        service_workspace::create_portal(&self.state, request).await
    }

    async fn get_entitlements(
        &self,
        request: Request<billing::GetEntitlementsRequest>,
    ) -> Result<Response<billing::EntitlementSnapshot>, Status> {
        service_workspace::get_entitlements(&self.state, request).await
    }

    async fn list_invoices(
        &self,
        request: Request<billing::ListInvoicesRequest>,
    ) -> Result<Response<billing::ListInvoicesResponse>, Status> {
        service_workspace::list_invoices(&self.state, request).await
    }

    async fn get_invoice(
        &self,
        request: Request<billing::GetInvoiceRequest>,
    ) -> Result<Response<billing::BillingInvoice>, Status> {
        service_workspace::get_invoice(&self.state, request).await
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

    async fn get_admin_tenant_billing_summary(
        &self,
        request: Request<billing::GetAdminTenantBillingSummaryRequest>,
    ) -> Result<Response<billing::AdminTenantBillingSummary>, Status> {
        let request = request.into_inner();
        validate_context(request.context.as_ref(), None)?;
        Ok(Response::new(
            service_admin::tenant_billing_summary(
                &self.state.db,
                parse_uuid(&request.tenant_id, "tenant_id")?,
            )
            .await?,
        ))
    }

    async fn get_admin_workspace_billing_summary(
        &self,
        request: Request<billing::GetAdminWorkspaceBillingSummaryRequest>,
    ) -> Result<Response<billing::AdminWorkspaceBillingSummary>, Status> {
        let request = request.into_inner();
        let workspace_id = workspace_id(&request.workspace_id)?;
        validate_context(request.context.as_ref(), Some(workspace_id))?;
        Ok(Response::new(
            service_admin::workspace_billing_summary(&self.state.db, workspace_id).await?,
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

    async fn run_admin_entitlements_action(
        &self,
        request: Request<billing::AdminEntitlementsActionRequest>,
    ) -> Result<Response<billing::AdminEntitlementsActionResult>, Status> {
        let request = request.into_inner();
        let workspace_id = workspace_id(&request.workspace_id)?;
        let context = validate_context(request.context.as_ref(), Some(workspace_id))?;
        Ok(Response::new(
            service_admin_entitlements::run_entitlements_action(
                &self.state.db,
                request.action_kind(),
                tenant_id(context)?,
                request.reason,
            )
            .await?,
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
