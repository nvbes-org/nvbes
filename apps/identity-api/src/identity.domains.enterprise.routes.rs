use crate::app::AppState;
use crate::domains::enterprise::service;
use crate::domains::enterprise::types::{
    EnterpriseAccessUpdateInput, EnterpriseAccessUpdateResponse, EnterpriseAuditEventsResponse,
    EnterpriseBillingResponse, EnterpriseContextResponse, EnterpriseDevelopersResponse,
    EnterpriseInvitationInput, EnterpriseInvitationsResponse, EnterpriseOverviewResponse,
    EnterprisePoliciesResponse, EnterpriseReactivateInput, EnterpriseSecurityResponse,
    EnterpriseSuspendInput, EnterpriseUsageResponse, EnterpriseUsersResponse,
    EnterpriseWorkspacesResponse,
};
use crate::http::error::AppError;
use crate::http::middleware::jwt::{AuthContext, jwt_auth_middleware};
use axum::{
    Json, Router,
    extract::{Extension, Path, State},
    routing::{get, patch, post},
};
use uuid::Uuid;

pub fn router(state: &AppState) -> Router<AppState> {
    Router::new()
        .route("/enterprise/context", get(get_context))
        .route("/enterprise/overview", get(get_overview))
        .route("/enterprise/users", get(list_users))
        .route("/enterprise/invitations", post(create_invitations))
        .route(
            "/enterprise/users/{userId}/access",
            patch(update_user_access),
        )
        .route("/enterprise/users/{userId}/suspend", post(suspend_user))
        .route(
            "/enterprise/users/{userId}/reactivate",
            post(reactivate_user),
        )
        .route("/enterprise/workspaces", get(list_workspaces))
        .route("/enterprise/developers", get(list_developers))
        .route("/enterprise/policies", get(list_policies))
        .route("/enterprise/security", get(get_security))
        .route("/enterprise/audit-events", get(list_audit_events))
        .route("/enterprise/billing", get(get_billing))
        .route("/enterprise/usage", get(get_usage))
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            jwt_auth_middleware,
        ))
}

async fn get_context(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<EnterpriseContextResponse>, AppError> {
    let tenant_id = require_tenant(&auth)?;
    Ok(Json(
        service::get_context(
            &state.db,
            auth.user_id,
            tenant_id,
            auth.organization_id,
            auth.workspace_id,
        )
        .await?,
    ))
}

async fn get_overview(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<EnterpriseOverviewResponse>, AppError> {
    let tenant_id = require_tenant(&auth)?;
    Ok(Json(
        service::get_overview(&state.db, auth.user_id, tenant_id).await?,
    ))
}

async fn list_users(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<EnterpriseUsersResponse>, AppError> {
    let tenant_id = require_tenant(&auth)?;
    Ok(Json(
        service::list_users(&state.db, auth.user_id, tenant_id).await?,
    ))
}

async fn create_invitations(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(input): Json<EnterpriseInvitationInput>,
) -> Result<Json<EnterpriseInvitationsResponse>, AppError> {
    let tenant_id = require_tenant(&auth)?;
    Ok(Json(
        service::create_invitations(&state.db, auth.user_id, tenant_id, input).await?,
    ))
}

async fn update_user_access(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(user_id): Path<Uuid>,
    Json(input): Json<EnterpriseAccessUpdateInput>,
) -> Result<Json<EnterpriseAccessUpdateResponse>, AppError> {
    let tenant_id = require_tenant(&auth)?;
    Ok(Json(
        service::update_user_access(&state.db, auth.user_id, tenant_id, user_id, input).await?,
    ))
}

async fn suspend_user(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(user_id): Path<Uuid>,
    Json(input): Json<EnterpriseSuspendInput>,
) -> Result<Json<EnterpriseAccessUpdateResponse>, AppError> {
    let tenant_id = require_tenant(&auth)?;
    Ok(Json(
        service::suspend_user(&state.db, auth.user_id, tenant_id, user_id, input).await?,
    ))
}

async fn reactivate_user(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(user_id): Path<Uuid>,
    Json(input): Json<EnterpriseReactivateInput>,
) -> Result<Json<EnterpriseAccessUpdateResponse>, AppError> {
    let tenant_id = require_tenant(&auth)?;
    Ok(Json(
        service::reactivate_user(&state.db, auth.user_id, tenant_id, user_id, input).await?,
    ))
}

async fn list_workspaces(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<EnterpriseWorkspacesResponse>, AppError> {
    let tenant_id = require_tenant(&auth)?;
    Ok(Json(
        service::list_workspaces(&state.db, auth.user_id, tenant_id).await?,
    ))
}

async fn list_developers(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<EnterpriseDevelopersResponse>, AppError> {
    let tenant_id = require_tenant(&auth)?;
    Ok(Json(
        service::list_developers(&state.db, auth.user_id, tenant_id).await?,
    ))
}

async fn list_policies(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<EnterprisePoliciesResponse>, AppError> {
    let tenant_id = require_tenant(&auth)?;
    Ok(Json(
        service::list_policies(&state.db, auth.user_id, tenant_id).await?,
    ))
}

async fn get_security(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<EnterpriseSecurityResponse>, AppError> {
    let tenant_id = require_tenant(&auth)?;
    Ok(Json(
        service::get_security(&state.db, auth.user_id, tenant_id).await?,
    ))
}

async fn list_audit_events(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<EnterpriseAuditEventsResponse>, AppError> {
    let tenant_id = require_tenant(&auth)?;
    Ok(Json(
        service::list_audit_events(&state.db, auth.user_id, tenant_id).await?,
    ))
}

async fn get_billing(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<EnterpriseBillingResponse>, AppError> {
    let tenant_id = require_tenant(&auth)?;
    Ok(Json(
        service::get_billing(&state.db, auth.user_id, tenant_id).await?,
    ))
}

async fn get_usage(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<EnterpriseUsageResponse>, AppError> {
    let tenant_id = require_tenant(&auth)?;
    Ok(Json(
        service::get_usage(&state.db, auth.user_id, tenant_id).await?,
    ))
}

fn require_tenant(auth: &AuthContext) -> Result<Uuid, AppError> {
    auth.tenant_id.ok_or_else(|| {
        AppError::bad_request(
            "missing_tenant",
            "Tenant ID is required for enterprise administration.",
        )
    })
}
