use axum::{Json, Router, extract::State, http::HeaderMap, routing::post};
use nvbes_core::http::error::ErrorEnvelope;
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
    axum::extract::Form(request): axum::extract::Form<RevokeRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let _client_auth = super::parse_basic_client_auth(&headers)?;

    super::enforce_public_oauth_rate_limit_db(
        &state.redis,
        &headers,
        "oauth_revoke",
        &_client_auth.0,
        60,
        30,
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

    let Ok(session_id) = Uuid::parse_str(&claims.sid) else {
        return Ok(Json(serde_json::json!({})));
    };
    let Ok(user_id) = Uuid::parse_str(&claims.sub) else {
        return Ok(Json(serde_json::json!({})));
    };

    if claims.token_type == "refresh" {
        nvbes_redis::refresh_token::revoke_refresh_token(&state.redis, &claims.jti)
            .await
            .map_err(|err| AppError::internal("refresh_token_revoke_failed", &err.to_string()))?;
    }

    revoke_refresh_family(&state.redis, user_id, session_id, &claims.jti).await?;

    Ok(Json(serde_json::json!({})))
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub(crate) struct RevokeRequest {
    token: String,
    token_type_hint: Option<String>,
}

async fn revoke_refresh_family(
    redis: &nvbes_redis::RedisPool,
    user_id: Uuid,
    session_id: Uuid,
    jti: &str,
) -> Result<(), AppError> {
    nvbes_redis::refresh_token::revoke_refresh_family(redis, user_id, session_id, jti)
        .await
        .map_err(|err| AppError::internal("refresh_token_revoke_failed", &err.to_string()))?;
    let _ =
        nvbes_redis::session::delete_session(redis, &user_id.to_string(), &session_id.to_string())
            .await;
    Ok(())
}
