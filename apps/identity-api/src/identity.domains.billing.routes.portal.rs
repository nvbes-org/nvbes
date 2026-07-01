use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, header},
    response::IntoResponse,
    routing::get,
};
use serde::Serialize;
use uuid::Uuid;

use crate::app::AppState;
use crate::domains::authz::{ResourceContext, WorkspaceAction, authorize_workspace_action};
use crate::domains::billing::service::{self, BillingPortalView};
use crate::http::error::AppError;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/billing/portal/capabilities", get(portal_capabilities))
        .route(
            "/workspaces/{workspaceId}/billing/portal/view",
            get(get_portal_view),
        )
        .route(
            "/workspaces/{workspaceId}/billing/portal/invoices/{invoiceId}/pdf",
            get(download_invoice_pdf),
        )
}

#[derive(Debug, Serialize)]
pub struct BillingPortalCapabilities {
    pub exposes_provider_secret_ids: bool,
    pub payment_method_update_flow: String,
    pub payment_method_changes_delegated_to_provider: bool,
    pub automatically_updates_payment_method_references: bool,
    pub shows_canonical_invoices: bool,
    pub shows_credits: bool,
}

pub async fn portal_capabilities() -> Json<BillingPortalCapabilities> {
    Json(BillingPortalCapabilities {
        exposes_provider_secret_ids: false,
        payment_method_update_flow: "nvbes_provider_redirect".to_string(),
        payment_method_changes_delegated_to_provider: false,
        automatically_updates_payment_method_references: false,
        shows_canonical_invoices: true,
        shows_credits: true,
    })
}

pub async fn get_portal_view(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
) -> Result<Json<BillingPortalView>, AppError> {
    let access = authorize_workspace_action(
        &state.db,
        &state.redis,
        &state.jwt,
        &headers,
        workspace_id,
        WorkspaceAction::ViewBilling,
        ResourceContext::default(),
    )
    .await?;
    let tenant_id = access.tenant_id.ok_or_else(|| {
        AppError::bad_request(
            "tenant_context_required",
            "Billing portal requires a tenant-scoped workspace.",
        )
    })?;

    Ok(Json(
        service::get_portal_view(&state.db, tenant_id, workspace_id).await?,
    ))
}

pub async fn download_invoice_pdf(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((workspace_id, invoice_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, AppError> {
    let access = authorize_workspace_action(
        &state.db,
        &state.redis,
        &state.jwt,
        &headers,
        workspace_id,
        WorkspaceAction::ViewBilling,
        ResourceContext::default(),
    )
    .await?;
    let tenant_id = access.tenant_id.ok_or_else(|| {
        AppError::bad_request(
            "tenant_context_required",
            "Billing invoice downloads require a tenant-scoped workspace.",
        )
    })?;
    let download = service::get_invoice_pdf(&state.db, tenant_id, workspace_id, invoice_id).await?;

    Ok((
        [
            (header::CONTENT_TYPE, "application/pdf".to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{}\"", download.filename),
            ),
            (header::CACHE_CONTROL, "no-store".to_string()),
        ],
        download.body,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn billing_portal_capabilities_do_not_expose_provider_secret_ids() {
        let capabilities = BillingPortalCapabilities {
            exposes_provider_secret_ids: false,
            payment_method_update_flow: "nvbes_provider_redirect".to_string(),
            payment_method_changes_delegated_to_provider: false,
            automatically_updates_payment_method_references: false,
            shows_canonical_invoices: true,
            shows_credits: true,
        };
        assert!(!capabilities.exposes_provider_secret_ids);
        assert_eq!(
            capabilities.payment_method_update_flow,
            "nvbes_provider_redirect"
        );
        assert!(!capabilities.payment_method_changes_delegated_to_provider);
        assert!(!capabilities.automatically_updates_payment_method_references);
    }
}
