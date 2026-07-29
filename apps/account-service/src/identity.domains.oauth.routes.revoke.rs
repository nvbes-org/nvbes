use axum::{Extension, Json, Router, extract::State, http::HeaderMap, routing::post};
use nvbes_core::http::error::ErrorEnvelope;
use std::time::Duration;
use uuid::Uuid;

use crate::{app::AppState, http::error::AppError};

pub fn router() -> Router<AppState> {
    Router::new().route("/revoke", post(revoke))
}

#[utoipa::path(
    post,
    path = "/oauth/revoke",
    tag = "oauth",
    responses(
        (status = 200, description = "Token revoked successfully"),
        (status = 400, description = "Bad request", body = ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn revoke(
    State(state): State<AppState>,
    headers: HeaderMap,
    dpop: Option<Extension<crate::http::middleware::dpop::DpopContext>>,
    mtls: Option<Extension<crate::http::mtls::MtlsCertificateThumbprint>>,
    axum::extract::Form(request): axum::extract::Form<RevokeRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let mut client_auth = super::token::token_client_auth(
        &headers,
        request.client_id.as_deref(),
        request.client_secret.as_deref(),
        request.client_assertion_type.as_deref(),
        request.client_assertion.as_deref(),
    )?;
    let endpoint = format!(
        "{}/oauth/revoke",
        state.config.api_base_url.trim_end_matches('/')
    );
    client_auth.client_assertion_verified =
        crate::domains::oauth::client_assertion::verify_private_key_jwt(
            &state.db,
            &client_auth,
            &endpoint,
            state.config.api_base_url.trim_end_matches('/'),
        )
        .await?;
    let security =
        crate::domains::oauth::profiles::load_client_security(&state.db, &client_auth.client_id)
            .await?;
    super::token::enforce_token_endpoint_security(
        &state,
        &client_auth,
        &security,
        dpop.as_ref().map(|Extension(context)| context),
        mtls.as_ref().map(|Extension(thumbprint)| thumbprint),
    )?;
    verify_client_secret_if_needed(&state, &client_auth, security.tenant_id).await?;

    nvbes_core::limiter::check_dual_rate_limit(
        &state.redis,
        &headers,
        "oauth_revoke",
        &client_auth.client_id,
        60,
        30,
        Duration::from_secs(60),
    )
    .await?;

    let token_type = request
        .token_type_hint
        .as_deref()
        .unwrap_or("refresh_token");

    let result = match token_type {
        "refresh_token" => state.jwt.decode_token(&request.token, "refresh"),
        "access_token" => state.jwt.decode_token(&request.token, "access"),
        _ => state
            .jwt
            .decode_token(&request.token, "refresh")
            .or_else(|_| state.jwt.decode_token(&request.token, "access")),
    };

    let Ok(claims) = result else {
        return Ok(Json(serde_json::json!({})));
    };
    if claims.client_id.as_deref() != Some(client_auth.client_id.as_str()) {
        return Ok(Json(serde_json::json!({})));
    }

    let Ok(session_id) = Uuid::parse_str(&claims.sid) else {
        return Ok(Json(serde_json::json!({})));
    };
    let Ok(user_id) = Uuid::parse_str(&claims.sub) else {
        return Ok(Json(serde_json::json!({})));
    };

    if claims.token_type == "refresh" {
        nvbes_redis::refresh_token::revoke_refresh_token(&state.redis, &claims.jti)
            .await
            .map_err(|err| AppError::internal("refresh_token_revoke_failed", err.to_string()))?;
    }

    revoke_refresh_family(&state.redis, user_id, session_id, &claims.jti).await?;

    Ok(Json(serde_json::json!({})))
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct RevokeRequest {
    token: String,
    token_type_hint: Option<String>,
    client_id: Option<String>,
    client_secret: Option<String>,
    client_assertion_type: Option<String>,
    client_assertion: Option<String>,
}

async fn verify_client_secret_if_needed(
    state: &AppState,
    auth: &crate::domains::oauth::service::ClientAuthentication,
    tenant_id: Uuid,
) -> Result<(), AppError> {
    if auth.client_assertion_verified {
        return Ok(());
    }
    let secret = auth.client_secret.as_deref().ok_or_else(|| {
        AppError::unauthorized("invalid_client", "Client authentication is required.")
    })?;
    let hash: String = sqlx::query_scalar(
        "SELECT client_secret_hash FROM oauth_clients WHERE client_id = $1 AND revoked_at IS NULL",
    )
    .bind(&auth.client_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::unauthorized("invalid_client", "The OAuth client is invalid."))?;
    crate::domains::oauth::verify_client_secret_with_overlap(
        tenant_id,
        &auth.client_id,
        secret,
        &hash,
    )
    .await
}

async fn revoke_refresh_family(
    redis: &nvbes_redis::RedisPool,
    user_id: Uuid,
    session_id: Uuid,
    jti: &str,
) -> Result<(), AppError> {
    nvbes_redis::refresh_token::revoke_refresh_family(redis, user_id, session_id, jti)
        .await
        .map_err(|err| AppError::internal("refresh_token_revoke_failed", err.to_string()))?;
    let _ =
        nvbes_redis::session::delete_session(redis, &user_id.to_string(), &session_id.to_string())
            .await;
    Ok(())
}
