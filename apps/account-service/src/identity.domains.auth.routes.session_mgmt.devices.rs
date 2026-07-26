use axum::{
    Json,
    extract::{Extension, Path, State},
    http::HeaderMap,
};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::app::AppState;
use crate::domains::auth::types::StepUpSubject;
use crate::domains::auth::{audit, device_trust, verification};
use crate::http::error::AppError;
use crate::http::middleware::jwt::AuthContext;

#[derive(Debug, Serialize, ToSchema)]
pub struct DeviceTrustResult {
    pub success: bool,
    pub device_id: Uuid,
    pub trust_level: String,
    pub trust_score: i16,
}

#[utoipa::path(
    post,
    path = "/auth/devices/{deviceId}/trust",
    tag = "auth",
    responses(
        (status = 200, description = "Device trusted after recent step-up", body = DeviceTrustResult),
        (status = 401, description = "Recent step-up required", body = nvbes_core::http::error::ErrorEnvelope),
        (status = 404, description = "Device not found", body = nvbes_core::http::error::ErrorEnvelope),
    ),
)]
pub(crate) async fn trust_device(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(device_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<DeviceTrustResult>, AppError> {
    verification::require_recent_step_up(&state.redis, &auth, None).await?;
    let mutation = device_trust::trust_device(&state.db, auth.user_id(), device_id).await?;
    sync_device_sessions(
        &state.redis,
        auth.user_id(),
        device_id,
        &mutation.trust_level,
        mutation.trust_score,
    )
    .await;
    record_device_audit(&state, &auth, &mutation, "auth.device_trusted", &headers).await;
    Ok(Json(result(mutation)))
}

#[utoipa::path(
    delete,
    path = "/auth/devices/{deviceId}",
    tag = "auth",
    responses(
        (status = 200, description = "Device revoked and its sessions terminated", body = DeviceTrustResult),
        (status = 401, description = "Recent step-up required", body = nvbes_core::http::error::ErrorEnvelope),
        (status = 404, description = "Device not found", body = nvbes_core::http::error::ErrorEnvelope),
    ),
)]
pub(crate) async fn revoke_device(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(device_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<DeviceTrustResult>, AppError> {
    verification::require_recent_step_up(&state.redis, &auth, None).await?;
    let mutation = device_trust::revoke_device(&state.db, auth.user_id(), device_id).await?;
    revoke_device_sessions(&state.redis, auth.user_id(), device_id).await?;
    record_device_audit(&state, &auth, &mutation, "auth.device_revoked", &headers).await;
    Ok(Json(result(mutation)))
}

async fn sync_device_sessions(
    redis: &nvbes_redis::RedisPool,
    principal_id: Uuid,
    device_id: Uuid,
    trust_level: &str,
    trust_score: i16,
) {
    let device_id = device_id.to_string();
    let Ok(session_ids) =
        nvbes_redis::session::list_user_sessions(redis, &principal_id.to_string()).await
    else {
        return;
    };
    for session_id in session_ids {
        let Ok(Some(mut session)) = nvbes_redis::session::get_session(redis, &session_id).await
        else {
            continue;
        };
        if session.account_device_id.as_deref() != Some(device_id.as_str()) {
            continue;
        }
        session.device_trust_level = Some(trust_level.to_string());
        session.device_trust_score = Some(trust_score);
        let ttl = crate::domains::auth::sessions::cache::current_session_ttl(&session);
        let _ = nvbes_redis::session::set_session(redis, &session, ttl).await;
    }
}

async fn revoke_device_sessions(
    redis: &nvbes_redis::RedisPool,
    principal_id: Uuid,
    device_id: Uuid,
) -> Result<(), AppError> {
    let device_id = device_id.to_string();
    let session_ids = nvbes_redis::session::list_user_sessions(redis, &principal_id.to_string())
        .await
        .map_err(|error| AppError::internal("device_sessions_read_failed", error.to_string()))?;
    for session_id in session_ids {
        let Ok(Some(session)) = nvbes_redis::session::get_session(redis, &session_id).await else {
            continue;
        };
        if session.account_device_id.as_deref() == Some(device_id.as_str()) {
            let Ok(session_uuid) = Uuid::parse_str(&session_id) else {
                continue;
            };
            let _ = nvbes_redis::refresh_token::revoke_session_refresh_tokens(
                redis,
                principal_id,
                session_uuid,
            )
            .await;
            let _ =
                nvbes_redis::session::delete_session(redis, &principal_id.to_string(), &session_id)
                    .await;
        }
    }
    Ok(())
}

async fn record_device_audit(
    state: &AppState,
    auth: &AuthContext,
    mutation: &device_trust::DeviceTrustMutation,
    action: &'static str,
    headers: &HeaderMap,
) {
    let ip = crate::http::request::client_ip(headers);
    let user_agent = crate::http::request::user_agent(headers);
    let _ = audit::record_auth_event(
        &state.db,
        audit::AuthAuditInput {
            principal_id: auth.user_id(),
            action,
            target_type: "account_device",
            target_id: Some(mutation.device_id),
            ip: ip.as_deref(),
            user_agent: user_agent.as_deref(),
            metadata: serde_json::json!({
                "trust_level": mutation.trust_level,
                "trust_score": mutation.trust_score,
                "revoked_at": mutation.revoked_at,
            }),
        },
    )
    .await;
}

fn result(mutation: device_trust::DeviceTrustMutation) -> DeviceTrustResult {
    DeviceTrustResult {
        success: true,
        device_id: mutation.device_id,
        trust_level: mutation.trust_level,
        trust_score: mutation.trust_score,
    }
}
