use crate::domains::auth::sessions;
use crate::{app::AppState, http::error::AppError};
use axum::{Json, Router, extract::State, http::HeaderMap, routing::get};
use nvbes_core::http::error::ErrorEnvelope;

pub fn router() -> Router<AppState> {
    Router::new().route("/userinfo", get(userinfo))
}

#[utoipa::path(
    get,
    path = "/oauth/userinfo",
    tag = "oauth",
    responses(
        (status = 200, description = "User info claims"),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn userinfo(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, AppError> {
    let token = crate::http::request::bearer_token(&headers)?;
    let auth = sessions::authenticate(&state.db, &state.redis, &state.jwt, &token).await?;
    super::enforce_public_oauth_rate_limit(
        &state.redis,
        &headers,
        "oauth_userinfo",
        &format!("session:{}", auth.session_id),
        120,
        120,
    )
    .await?;
    let assurance = crate::domains::oauth::assurance::resolve_assurance_context(
        &state.db,
        &state.redis,
        auth.user_id,
        Some(auth.session_id),
        auth.client_id.as_deref(),
        auth.tenant_id,
        auth.organization_id,
        auth.workspace_id,
    )
    .await?;
    if !assurance.sufficient {
        return Err(AppError::forbidden(
            "assurance_level_insufficient",
            "The current session assurance level is insufficient for this context.",
        ));
    }

    Ok(Json(serde_json::json!({
        "sub": auth.user_id,
        "sid": auth.session_id,
        "email": auth.user_email,
        "email_verified": auth.email_verified_at.is_some(),
        "display_name": auth.display_name,
        "tenant_id": auth.tenant_id,
        "organization_id": auth.organization_id,
        "workspace_id": auth.workspace_id,
        "scope": auth.scope,
        "acr": assurance.acr,
        "amr": assurance.amr,
        "auth_time": assurance.auth_time,
        "client_id": auth.client_id,
        "preferred_username": auth.user_email,
        "updated_at": auth.email_verified_at.map(|value| value.timestamp()).unwrap_or_default(),
    })))
}
