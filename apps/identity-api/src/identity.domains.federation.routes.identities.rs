use super::authenticate_tenant;
use crate::app::AppState;
use crate::domains::federation::types::{
    CreateLinkedIdentityInput, JitProvisioningInput, JitProvisioningResponse,
    LinkedIdentitiesResponse,
};
use crate::http::error::AppError;
use axum::{
    Json, Router,
    extract::{Path, State},
    http::HeaderMap,
    routing::{delete, get, post},
};
use nvbes_core::http::error::ErrorEnvelope;
use serde::Deserialize;
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/tenants/{tenantId}/linked-identities",
            get(list_linked_identities).post(link_identity),
        )
        .route(
            "/tenants/{tenantId}/linked-identities/{identityId}",
            delete(unlink_identity),
        )
        .route("/tenants/{tenantId}/jit-provisioning", post(jit_provision))
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) struct CreateLinkedIdentityRequest {
    principal_id: Uuid,
    provider_type: String,
    provider_id: String,
    subject: String,
    email: Option<String>,
    email_verified: Option<bool>,
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) struct JitProvisioningRequest {
    email: String,
    username: String,
    provider_type: String,
    provider_id: String,
    subject: String,
    email_verified: Option<bool>,
}

#[utoipa::path(
    get,
    path = "/tenants/{tenantId}/linked-identities",
    tag = "federation",
    params(
        ("tenantId" = Uuid, Path, description = "Tenant ID"),
    ),
    responses(
        (status = 200, description = "List linked identities", body = LinkedIdentitiesResponse),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn list_linked_identities(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(tenant_id): Path<Uuid>,
) -> Result<Json<LinkedIdentitiesResponse>, AppError> {
    let _auth = authenticate_tenant(&state, &headers, tenant_id).await?;
    Ok(Json(
        crate::domains::federation::identities::list_linked_identities(&state.db, tenant_id)
            .await?,
    ))
}

#[utoipa::path(
    post,
    path = "/tenants/{tenantId}/linked-identities",
    tag = "federation",
    params(
        ("tenantId" = Uuid, Path, description = "Tenant ID"),
    ),
    request_body = CreateLinkedIdentityRequest,
    responses(
        (status = 200, description = "Identity linked", body = crate::domains::federation::types::LinkedIdentityResponse),
        (status = 400, description = "Bad request", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn link_identity(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(tenant_id): Path<Uuid>,
    Json(request): Json<CreateLinkedIdentityRequest>,
) -> Result<Json<crate::domains::federation::types::LinkedIdentityResponse>, AppError> {
    let _auth = authenticate_tenant(&state, &headers, tenant_id).await?;
    Ok(Json(
        crate::domains::federation::identities::link_identity(
            &state.db,
            tenant_id,
            CreateLinkedIdentityInput {
                principal_id: request.principal_id,
                provider_type: request.provider_type,
                provider_id: request.provider_id,
                subject: request.subject,
                email: request.email,
                email_verified: request.email_verified.unwrap_or(true),
            },
        )
        .await?,
    ))
}

#[utoipa::path(
    delete,
    path = "/tenants/{tenantId}/linked-identities/{identityId}",
    tag = "federation",
    params(
        ("tenantId" = Uuid, Path, description = "Tenant ID"),
        ("identityId" = Uuid, Path, description = "Identity ID"),
    ),
    responses(
        (status = 200, description = "Identity unlinked"),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 404, description = "Not found", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn unlink_identity(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((tenant_id, identity_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let _auth = authenticate_tenant(&state, &headers, tenant_id).await?;
    crate::domains::federation::identities::unlink_identity(&state.db, tenant_id, identity_id)
        .await?;
    Ok(Json(serde_json::json!({ "success": true })))
}

#[utoipa::path(
    post,
    path = "/tenants/{tenantId}/jit-provisioning",
    tag = "federation",
    params(
        ("tenantId" = Uuid, Path, description = "Tenant ID"),
    ),
    request_body = JitProvisioningRequest,
    responses(
        (status = 200, description = "JIT provisioning completed", body = JitProvisioningResponse),
        (status = 400, description = "Bad request", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn jit_provision(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(tenant_id): Path<Uuid>,
    Json(request): Json<JitProvisioningRequest>,
) -> Result<Json<JitProvisioningResponse>, AppError> {
    let _auth = authenticate_tenant(&state, &headers, tenant_id).await?;
    Ok(Json(
        crate::domains::federation::provisioning::jit_provision(
            &state.db,
            tenant_id,
            JitProvisioningInput {
                email: request.email,
                username: request.username,
                provider_type: request.provider_type,
                provider_id: request.provider_id,
                subject: request.subject,
                email_verified: request.email_verified.unwrap_or(true),
            },
        )
        .await?,
    ))
}
