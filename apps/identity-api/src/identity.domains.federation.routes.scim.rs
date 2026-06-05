use super::authenticate_tenant;
use crate::app::AppState;
use crate::domains::federation::types::{
    CreateScimProvisioningConnectorInput, ScimProvisioningConnectorsResponse,
    UpdateScimProvisioningConnectorInput,
};
use crate::http::error::AppError;
use axum::{
    Json, Router,
    extract::{Path, State},
    http::HeaderMap,
    routing::{get, patch},
};
use nvbes_core::http::error::ErrorEnvelope;
use serde::Deserialize;
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/tenants/{tenantId}/scim-connectors",
            get(list_scim_connectors).post(create_scim_connector),
        )
        .route(
            "/tenants/{tenantId}/scim-connectors/{connectorId}",
            patch(update_scim_connector).delete(delete_scim_connector),
        )
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) struct CreateScimProvisioningConnectorRequest {
    provider: String,
    base_url: Option<String>,
    status: Option<String>,
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) struct UpdateScimProvisioningConnectorRequest {
    provider: Option<String>,
    base_url: Option<String>,
    status: Option<String>,
}

#[utoipa::path(
    get,
    path = "/tenants/{tenantId}/scim-connectors",
    tag = "federation",
    params(
        ("tenantId" = Uuid, Path, description = "Tenant ID"),
    ),
    responses(
        (status = 200, description = "List SCIM connectors", body = ScimProvisioningConnectorsResponse),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn list_scim_connectors(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(tenant_id): Path<Uuid>,
) -> Result<Json<ScimProvisioningConnectorsResponse>, AppError> {
    let _auth = authenticate_tenant(&state, &headers, tenant_id).await?;
    Ok(Json(
        crate::domains::federation::scim::list_scim_connectors(&state.db, tenant_id).await?,
    ))
}

#[utoipa::path(
    post,
    path = "/tenants/{tenantId}/scim-connectors",
    tag = "federation",
    params(
        ("tenantId" = Uuid, Path, description = "Tenant ID"),
    ),
    request_body = CreateScimProvisioningConnectorRequest,
    responses(
        (status = 200, description = "SCIM connector created", body = crate::domains::federation::types::ScimProvisioningConnectorResponse),
        (status = 400, description = "Bad request", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn create_scim_connector(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(tenant_id): Path<Uuid>,
    Json(request): Json<CreateScimProvisioningConnectorRequest>,
) -> Result<Json<crate::domains::federation::types::ScimProvisioningConnectorResponse>, AppError> {
    let _auth = authenticate_tenant(&state, &headers, tenant_id).await?;
    Ok(Json(
        crate::domains::federation::scim::create_scim_connector(
            &state.db,
            tenant_id,
            CreateScimProvisioningConnectorInput {
                provider: request.provider,
                base_url: request.base_url,
                status: request.status,
            },
        )
        .await?,
    ))
}

#[utoipa::path(
    patch,
    path = "/tenants/{tenantId}/scim-connectors/{connectorId}",
    tag = "federation",
    params(
        ("tenantId" = Uuid, Path, description = "Tenant ID"),
        ("connectorId" = Uuid, Path, description = "Connector ID"),
    ),
    request_body = UpdateScimProvisioningConnectorRequest,
    responses(
        (status = 200, description = "SCIM connector updated", body = crate::domains::federation::types::ScimProvisioningConnectorResponse),
        (status = 400, description = "Bad request", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 404, description = "Not found", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn update_scim_connector(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((tenant_id, connector_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<UpdateScimProvisioningConnectorRequest>,
) -> Result<Json<crate::domains::federation::types::ScimProvisioningConnectorResponse>, AppError> {
    let _auth = authenticate_tenant(&state, &headers, tenant_id).await?;
    Ok(Json(
        crate::domains::federation::scim::update_scim_connector(
            &state.db,
            tenant_id,
            connector_id,
            UpdateScimProvisioningConnectorInput {
                provider: request.provider,
                base_url: request.base_url,
                status: request.status,
            },
        )
        .await?,
    ))
}

#[utoipa::path(
    delete,
    path = "/tenants/{tenantId}/scim-connectors/{connectorId}",
    tag = "federation",
    params(
        ("tenantId" = Uuid, Path, description = "Tenant ID"),
        ("connectorId" = Uuid, Path, description = "Connector ID"),
    ),
    responses(
        (status = 200, description = "SCIM connector deleted"),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 404, description = "Not found", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn delete_scim_connector(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((tenant_id, connector_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let _auth = authenticate_tenant(&state, &headers, tenant_id).await?;
    crate::domains::federation::scim::delete_scim_connector(&state.db, tenant_id, connector_id)
        .await?;
    Ok(Json(serde_json::json!({ "success": true })))
}
