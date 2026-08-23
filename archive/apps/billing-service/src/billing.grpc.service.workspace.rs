use tonic::{Request, Response, Status};

use crate::{
    app::BillingAppState,
    grpc::{
        pb::nvbes::billing::v1 as billing,
        service_conversions,
        service_status::{
            checkout_status, empty_to_none, parse_uuid, portal_status, sql_status,
            validate_context, workspace_id,
        },
    },
};

pub(super) async fn get_billing_overview(
    state: &BillingAppState,
    request: Request<billing::GetBillingOverviewRequest>,
) -> Result<Response<billing::BillingOverview>, Status> {
    let request = request.into_inner();
    let workspace_id = workspace_id(&request.workspace_id)?;
    validate_context(request.context.as_ref(), Some(workspace_id))?;
    let overview = nvbes_billing::fetch_workspace_billing_overview(&state.db, workspace_id)
        .await
        .map_err(sql_status)?;
    Ok(Response::new(
        service_conversions::billing_overview_response(overview),
    ))
}

pub(super) async fn get_billing_portal(
    state: &BillingAppState,
    request: Request<billing::GetBillingPortalRequest>,
) -> Result<Response<billing::BillingPortal>, Status> {
    let request = request.into_inner();
    let workspace_id = workspace_id(&request.workspace_id)?;
    validate_context(request.context.as_ref(), Some(workspace_id))?;
    let portal = nvbes_billing::portal_views::fetch_portal_view(&state.db, workspace_id)
        .await
        .map_err(sql_status)?;
    Ok(Response::new(service_conversions::billing_portal_view(
        workspace_id,
        portal,
    )))
}

pub(super) async fn create_checkout(
    state: &BillingAppState,
    request: Request<billing::CreateCheckoutRequest>,
) -> Result<Response<billing::CheckoutSession>, Status> {
    let request = request.into_inner();
    let workspace_id = workspace_id(&request.workspace_id)?;
    let context = validate_context(request.context.as_ref(), Some(workspace_id))?;
    let actor_principal_id = parse_uuid(&context.actor_principal_id, "actor_principal_id")?;
    let plan_code = request.plan_code.clone();
    let checkout = nvbes_billing::checkout_sessions::create_billing_checkout_session(
        &state.db,
        &state.config,
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
    state.product_analytics.capture(
        nvbes_product_analytics::ProductAnalyticsEvent::workspace(
            "billing.checkout_started",
            workspace_id,
        )
        .property("plan_code", plan_code),
    );
    Ok(Response::new(
        service_conversions::checkout_session_response(checkout),
    ))
}

pub(super) async fn create_portal(
    state: &BillingAppState,
    request: Request<billing::CreatePortalRequest>,
) -> Result<Response<billing::PortalSession>, Status> {
    let request = request.into_inner();
    let workspace_id = workspace_id(&request.workspace_id)?;
    let context = validate_context(request.context.as_ref(), Some(workspace_id))?;
    let actor_principal_id = parse_uuid(&context.actor_principal_id, "actor_principal_id")?;
    let portal = nvbes_billing::portal_actions::create_billing_portal_session(
        &state.db,
        &state.config,
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

pub(super) async fn get_entitlements(
    state: &BillingAppState,
    request: Request<billing::GetEntitlementsRequest>,
) -> Result<Response<billing::EntitlementSnapshot>, Status> {
    let request = request.into_inner();
    let workspace_id = workspace_id(&request.workspace_id)?;
    validate_context(request.context.as_ref(), Some(workspace_id))?;
    let overview = nvbes_billing::fetch_workspace_billing_overview(&state.db, workspace_id)
        .await
        .map_err(sql_status)?;
    Ok(Response::new(service_conversions::entitlement_snapshot(
        overview,
    )))
}

pub(super) async fn list_invoices(
    state: &BillingAppState,
    request: Request<billing::ListInvoicesRequest>,
) -> Result<Response<billing::ListInvoicesResponse>, Status> {
    let request = request.into_inner();
    let workspace_id = workspace_id(&request.workspace_id)?;
    validate_context(request.context.as_ref(), Some(workspace_id))?;
    let invoices = nvbes_billing::portal_views::fetch_portal_invoices(&state.db, workspace_id)
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

pub(super) async fn get_invoice(
    state: &BillingAppState,
    request: Request<billing::GetInvoiceRequest>,
) -> Result<Response<billing::BillingInvoice>, Status> {
    let request = request.into_inner();
    let workspace_id = workspace_id(&request.workspace_id)?;
    validate_context(request.context.as_ref(), Some(workspace_id))?;
    let invoice_id = parse_uuid(&request.invoice_id, "invoice_id")?.to_string();
    let invoice = nvbes_billing::portal_views::fetch_portal_invoices(&state.db, workspace_id)
        .await
        .map_err(sql_status)?
        .into_iter()
        .map(service_conversions::billing_invoice)
        .find(|invoice| invoice.invoice_id == invoice_id)
        .ok_or_else(|| Status::not_found("billing invoice was not found"))?;
    Ok(Response::new(invoice))
}
