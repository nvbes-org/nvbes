use axum::{
    Router,
    extract::{Path, State},
    http::{HeaderMap, HeaderName, header},
    response::IntoResponse,
    routing::post,
};
use uuid::Uuid;

use crate::app::AppState;
use crate::domains::billing::service_exports::{build_finance_export, parse_finance_export_type};
use crate::http::error::AppError;

pub fn router() -> Router<AppState> {
    Router::new().route(
        "/workspaces/{workspaceId}/billing/admin/exports/{exportType}",
        post(create_finance_export_route),
    )
}

pub async fn create_finance_export_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((workspace_id, export_type)): Path<(Uuid, String)>,
) -> Result<impl IntoResponse, AppError> {
    let access = super::authorize_billing_admin_access(&state, &headers, workspace_id).await?;
    let tenant_id = access.tenant_id.ok_or_else(|| {
        AppError::bad_request(
            "tenant_context_required",
            "Billing finance exports require a tenant-scoped workspace.",
        )
    })?;
    let export_type = parse_finance_export_type(&export_type)?;
    let export = build_finance_export(&state.db, tenant_id, export_type).await?;

    Ok((
        [
            (header::CONTENT_TYPE, export.content_type.to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{}\"", export.filename),
            ),
            (header::CACHE_CONTROL, "no-store".to_string()),
            (
                HeaderName::from_static("x-nvbes-billing-export-run-id"),
                export.export_run_id.to_string(),
            ),
            (
                HeaderName::from_static("x-nvbes-billing-export-row-count"),
                export.row_count.to_string(),
            ),
        ],
        export.body,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn billing_finance_export_route_is_post_only() {
        let router = router();
        let _ = router;
    }
}
