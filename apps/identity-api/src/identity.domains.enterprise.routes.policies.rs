use axum::{
    Json,
    extract::{Extension, State},
};
use nvbes_core::http::error::ErrorEnvelope;

use crate::app::AppState;
use crate::domains::enterprise::policy_simulation::{
    EnterprisePolicySimulationInput, EnterprisePolicySimulationResponse, simulate_policy,
};
use crate::domains::enterprise::service;
use crate::domains::enterprise::types::{
    EnterpriseMfaPolicyInput, EnterprisePoliciesResponse, EnterpriseSessionPolicyInput,
};
use crate::http::error::AppError;
use crate::http::middleware::jwt::AuthContext;

use super::require_tenant;

pub(super) async fn list_policies(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<EnterprisePoliciesResponse>, AppError> {
    let tenant_id = require_tenant(&auth)?;
    Ok(Json(
        service::list_policies(
            &state.db,
            &auth,
            tenant_id,
            state.config.auth_session_ttl_hours,
        )
        .await?,
    ))
}

pub(super) async fn simulate_policy_decision(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(input): Json<EnterprisePolicySimulationInput>,
) -> Result<Json<EnterprisePolicySimulationResponse>, AppError> {
    let tenant_id = require_tenant(&auth)?;
    Ok(Json(
        simulate_policy(&state.db, &auth, tenant_id, input).await?,
    ))
}

pub(super) async fn update_session_policy(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(input): Json<EnterpriseSessionPolicyInput>,
) -> Result<Json<EnterprisePoliciesResponse>, AppError> {
    let tenant_id = require_tenant(&auth)?;
    Ok(Json(
        service::update_session_policy(
            &state.db,
            &state.redis,
            &auth,
            tenant_id,
            state.config.auth_session_ttl_hours,
            input,
        )
        .await?,
    ))
}

#[utoipa::path(
    patch,
    path = "/enterprise/policies/mfa",
    tag = "enterprise",
    request_body = EnterpriseMfaPolicyInput,
    responses(
        (status = 200, description = "MFA policy updated", body = EnterprisePoliciesResponse),
        (status = 400, description = "Bad request", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 403, description = "Policies grant or admin elevation required", body = ErrorEnvelope),
    ),
)]
pub async fn update_mfa_policy(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(input): Json<EnterpriseMfaPolicyInput>,
) -> Result<Json<EnterprisePoliciesResponse>, AppError> {
    let tenant_id = require_tenant(&auth)?;
    Ok(Json(
        service::update_mfa_policy(
            &state.db,
            &state.redis,
            &auth,
            tenant_id,
            state.config.auth_session_ttl_hours,
            input,
        )
        .await?,
    ))
}
