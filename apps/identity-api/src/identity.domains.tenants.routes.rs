use axum::{
    Json, Router,
    extract::{Path, State},
    http::HeaderMap,
    routing::{get, post, put},
};
use uuid::Uuid;

use super::{service, types::*};
use crate::{app::AppState, http::error::AppError};

pub fn router(_state: &AppState) -> Router<AppState> {
    Router::new()
        .route("/tenants/{tenantId}", get(get_tenant).patch(update_tenant))
        .route(
            "/tenants/{tenantId}/organizations",
            get(list_organizations).post(create_organization),
        )
        .route(
            "/tenants/{tenantId}/organizations/{organizationId}",
            axum::routing::patch(update_organization),
        )
        .route(
            "/tenants/{tenantId}/members/{principalId}",
            put(upsert_tenant_membership),
        )
        .route(
            "/tenants/{tenantId}/invitations",
            post(invite_tenant_member),
        )
        .route(
            "/tenants/{tenantId}/organizations/{organizationId}/members/{principalId}",
            put(upsert_organization_membership),
        )
        .route(
            "/tenants/{tenantId}/organizations/{organizationId}/invitations",
            post(invite_organization_member),
        )
        .route(
            "/identity-invitations/accept",
            post(accept_identity_invitation),
        )
}

async fn authenticate(
    state: &AppState,
    headers: &HeaderMap,
    tenant_id: Uuid,
) -> Result<crate::domains::auth::types::AuthContext, AppError> {
    crate::domains::federation::routes::authenticate_tenant(state, headers, tenant_id).await
}

pub(crate) async fn get_tenant(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(tenant_id): Path<Uuid>,
) -> Result<Json<TenantResponse>, AppError> {
    authenticate(&state, &headers, tenant_id).await?;
    Ok(Json(service::get_tenant(&state.db, tenant_id).await?))
}

pub(crate) async fn update_tenant(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(tenant_id): Path<Uuid>,
    Json(input): Json<UpdateTenantInput>,
) -> Result<Json<TenantResponse>, AppError> {
    authenticate(&state, &headers, tenant_id).await?;
    Ok(Json(
        service::update_tenant(&state.db, tenant_id, input).await?,
    ))
}

pub(crate) async fn list_organizations(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(tenant_id): Path<Uuid>,
) -> Result<Json<OrganizationsResponse>, AppError> {
    authenticate(&state, &headers, tenant_id).await?;
    Ok(Json(
        service::list_organizations(&state.db, tenant_id).await?,
    ))
}

pub(crate) async fn create_organization(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(tenant_id): Path<Uuid>,
    Json(input): Json<CreateOrganizationInput>,
) -> Result<Json<OrganizationResponse>, AppError> {
    let auth = authenticate(&state, &headers, tenant_id).await?;
    Ok(Json(
        service::create_organization(&state.db, tenant_id, auth.user_id, input).await?,
    ))
}

pub(crate) async fn update_organization(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((tenant_id, organization_id)): Path<(Uuid, Uuid)>,
    Json(input): Json<UpdateOrganizationInput>,
) -> Result<Json<OrganizationResponse>, AppError> {
    authenticate(&state, &headers, tenant_id).await?;
    Ok(Json(
        service::update_organization(&state.db, tenant_id, organization_id, input).await?,
    ))
}

pub(crate) async fn upsert_tenant_membership(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((tenant_id, principal_id)): Path<(Uuid, Uuid)>,
    Json(input): Json<UpsertIdentityMembershipInput>,
) -> Result<Json<IdentityMembershipResponse>, AppError> {
    authenticate(&state, &headers, tenant_id).await?;
    Ok(Json(
        service::upsert_tenant_membership(&state.db, tenant_id, principal_id, input).await?,
    ))
}

pub(crate) async fn invite_tenant_member(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(tenant_id): Path<Uuid>,
    Json(input): Json<InviteIdentityMemberInput>,
) -> Result<Json<IdentityInvitationResponse>, AppError> {
    let auth = authenticate(&state, &headers, tenant_id).await?;
    Ok(Json(
        service::invite_tenant_member(&state.db, &state.config, tenant_id, auth.user_id, input)
            .await?,
    ))
}

pub(crate) async fn upsert_organization_membership(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((tenant_id, organization_id, principal_id)): Path<(Uuid, Uuid, Uuid)>,
    Json(input): Json<UpsertIdentityMembershipInput>,
) -> Result<Json<IdentityMembershipResponse>, AppError> {
    authenticate(&state, &headers, tenant_id).await?;
    service::ensure_organization_in_tenant(&state.db, tenant_id, organization_id).await?;
    Ok(Json(
        service::upsert_organization_membership(&state.db, organization_id, principal_id, input)
            .await?,
    ))
}

pub(crate) async fn invite_organization_member(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((tenant_id, organization_id)): Path<(Uuid, Uuid)>,
    Json(input): Json<InviteIdentityMemberInput>,
) -> Result<Json<IdentityInvitationResponse>, AppError> {
    let auth = authenticate(&state, &headers, tenant_id).await?;
    service::ensure_organization_in_tenant(&state.db, tenant_id, organization_id).await?;
    Ok(Json(
        service::invite_organization_member(
            &state.db,
            &state.config,
            organization_id,
            auth.user_id,
            input,
        )
        .await?,
    ))
}

pub(crate) async fn accept_identity_invitation(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<AcceptIdentityInvitationInput>,
) -> Result<Json<AcceptIdentityInvitationResponse>, AppError> {
    let auth =
        crate::domains::auth::sessions::authenticate_verified_bearer(
            &state.db,
            &state.redis,
            &state.jwt,
            &headers,
        )
        .await?;
    Ok(Json(
        service::accept_identity_invitation(&state.db, &auth, input).await?,
    ))
}
