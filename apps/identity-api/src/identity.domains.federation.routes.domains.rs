use super::authenticate_tenant;
use crate::app::AppState;
use crate::domains::federation::types::{
    CreateTenantDomainInput, TenantDomainResponse, TenantDomainsResponse, VerifyTenantDomainInput,
    UpdateTenantDomainInput,
};
use crate::http::error::AppError;
use axum::{
    Json, Router,
    extract::{Path, State},
    http::HeaderMap,
    routing::{get, patch, post},
};
use nvbes_core::http::error::ErrorEnvelope;
use serde::Deserialize;
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/tenants/{tenantId}/domains",
            get(list_domains).post(create_domain),
        )
        .route(
            "/tenants/{tenantId}/domains/{domainId}/verify",
            post(verify_domain),
        )
        .route(
            "/tenants/{tenantId}/domains/{domainId}",
            patch(update_domain).delete(delete_domain),
        )
}

#[derive(Deserialize, utoipa::ToSchema)]
pub(crate) struct CreateTenantDomainRequest {
    domain: String,
    sso_required: Option<bool>,
    sso_provider_id: Option<Uuid>,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub(crate) struct UpdateTenantDomainRequest {
    sso_required: Option<bool>,
    sso_provider_id: Option<Uuid>,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub(crate) struct VerifyTenantDomainRequest {
    token: String,
}

#[utoipa::path(
    get,
    path = "/tenants/{tenantId}/domains",
    tag = "federation",
    params(
        ("tenantId" = Uuid, Path, description = "Tenant ID"),
    ),
    responses(
        (status = 200, description = "List tenant domains", body = TenantDomainsResponse),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn list_domains(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(tenant_id): Path<Uuid>,
) -> Result<Json<TenantDomainsResponse>, AppError> {
    let _auth = authenticate_tenant(&state, &headers, tenant_id).await?;
    Ok(Json(
        crate::domains::federation::domains::list_tenant_domains(&state.db, tenant_id).await?,
    ))
}

#[utoipa::path(
    post,
    path = "/tenants/{tenantId}/domains",
    tag = "federation",
    params(
        ("tenantId" = Uuid, Path, description = "Tenant ID"),
    ),
    request_body = CreateTenantDomainRequest,
    responses(
        (status = 200, description = "Domain created", body = TenantDomainResponse),
        (status = 400, description = "Bad request", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn create_domain(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(tenant_id): Path<Uuid>,
    Json(request): Json<CreateTenantDomainRequest>,
) -> Result<Json<TenantDomainResponse>, AppError> {
    let _auth = authenticate_tenant(&state, &headers, tenant_id).await?;
    Ok(Json(
        crate::domains::federation::domains::create_tenant_domain(
            &state.db,
            tenant_id,
            CreateTenantDomainInput {
                domain: request.domain,
                sso_required: request.sso_required,
                sso_provider_id: request.sso_provider_id,
            },
        )
        .await?,
    ))
}

#[utoipa::path(
    patch,
    path = "/tenants/{tenantId}/domains/{domainId}",
    tag = "federation",
    params(
        ("tenantId" = Uuid, Path, description = "Tenant ID"),
        ("domainId" = Uuid, Path, description = "Domain ID"),
    ),
    request_body = UpdateTenantDomainRequest,
    responses(
        (status = 200, description = "Domain policy updated", body = TenantDomainResponse),
        (status = 400, description = "Bad request", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 404, description = "Not found", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn update_domain(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((tenant_id, domain_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<UpdateTenantDomainRequest>,
) -> Result<Json<TenantDomainResponse>, AppError> {
    let _auth = authenticate_tenant(&state, &headers, tenant_id).await?;
    Ok(Json(
        crate::domains::federation::domains::update_tenant_domain(
            &state.db,
            tenant_id,
            domain_id,
            UpdateTenantDomainInput {
                sso_required: request.sso_required,
                sso_provider_id: request.sso_provider_id,
            },
        )
        .await?,
    ))
}

#[utoipa::path(
    post,
    path = "/tenants/{tenantId}/domains/{domainId}/verify",
    tag = "federation",
    params(
        ("tenantId" = Uuid, Path, description = "Tenant ID"),
        ("domainId" = Uuid, Path, description = "Domain ID"),
    ),
    request_body = VerifyTenantDomainRequest,
    responses(
        (status = 200, description = "Domain verified", body = TenantDomainResponse),
        (status = 400, description = "Bad request", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 404, description = "Not found", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn verify_domain(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((tenant_id, domain_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<VerifyTenantDomainRequest>,
) -> Result<Json<TenantDomainResponse>, AppError> {
    let _auth = authenticate_tenant(&state, &headers, tenant_id).await?;
    Ok(Json(
        crate::domains::federation::domains::verify_tenant_domain(
            &state.db,
            tenant_id,
            domain_id,
            VerifyTenantDomainInput {
                token: request.token,
            },
        )
        .await?,
    ))
}

#[utoipa::path(
    delete,
    path = "/tenants/{tenantId}/domains/{domainId}",
    tag = "federation",
    params(
        ("tenantId" = Uuid, Path, description = "Tenant ID"),
        ("domainId" = Uuid, Path, description = "Domain ID"),
    ),
    responses(
        (status = 200, description = "Domain deleted"),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 404, description = "Not found", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn delete_domain(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((tenant_id, domain_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let _auth = authenticate_tenant(&state, &headers, tenant_id).await?;
    crate::domains::federation::domains::delete_tenant_domain(&state.db, tenant_id, domain_id)
        .await?;
    Ok(Json(serde_json::json!({ "success": true })))
}
