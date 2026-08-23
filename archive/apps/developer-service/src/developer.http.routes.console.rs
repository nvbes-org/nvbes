use std::collections::BTreeSet;

use axum::{Extension, Json, extract::State};

use crate::{
    access,
    app::DeveloperAppState,
    grpc::{
        overview,
        pb::nvbes::developer::v1::{DeveloperOverviewSummary, GetOverviewSummaryRequest},
    },
    http::{
        auth::DeveloperAuth,
        context::{request_context, uuid},
        error::AppError,
        types::{DeveloperContextResponse, DeveloperOverviewResponse},
    },
    rbac::permissions_for_role,
};

#[utoipa::path(get, path = "/developer/console/context", tag = "developer-console", responses((status = 200, description = "Developer context")))]
pub async fn context(
    State(state): State<DeveloperAppState>,
    Extension(auth): Extension<DeveloperAuth>,
) -> Result<Json<DeveloperContextResponse>, AppError> {
    let roles = access::roles(&state.db, &auth).await?;
    let permissions = roles
        .iter()
        .copied()
        .flat_map(permissions_for_role)
        .map(|permission| permission.as_api_str().to_string())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    Ok(Json(DeveloperContextResponse {
        tenant_id: auth.tenant_id,
        principal_id: auth.user_id,
        display_name: auth.display_name,
        email: auth.email,
        roles: roles
            .into_iter()
            .map(|role| role.as_db_str().to_string())
            .collect(),
        permissions,
    }))
}

#[utoipa::path(get, path = "/developer/console/overview", tag = "developer-console", responses((status = 200, description = "Developer overview")))]
pub async fn overview(
    State(state): State<DeveloperAppState>,
    Extension(auth): Extension<DeveloperAuth>,
) -> Result<Json<DeveloperOverviewResponse>, AppError> {
    let response = overview::overview_summary(
        &state.db,
        GetOverviewSummaryRequest {
            context: Some(request_context(&auth)),
            tenant_id: auth.tenant_id.to_string(),
        },
    )
    .await?;
    Ok(Json(overview_response(response)?))
}

fn overview_response(
    response: DeveloperOverviewSummary,
) -> Result<DeveloperOverviewResponse, AppError> {
    Ok(DeveloperOverviewResponse {
        tenant_id: uuid(&response.tenant_id, "tenant id")?,
        oauth_clients: response.oauth_clients,
        marketplace_pending: response.marketplace_pending,
        high_risk_scopes: response.high_risk_scopes,
        failed_webhook_deliveries: response.failed_webhook_deliveries,
        unhealthy_integrations: response.unhealthy_integrations,
        active_sandboxes: response.active_sandboxes,
    })
}
