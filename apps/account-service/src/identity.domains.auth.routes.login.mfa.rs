use crate::app::AppState;
use crate::domains::auth::audit::{AuthAuditInput, record_auth_event};
use crate::domains::auth::sessions;
use crate::domains::auth::state::delete_state;
use crate::http::error::AppError;
use axum::{
    Json, Router,
    extract::{Query, State},
    http::HeaderMap,
    response::Response,
    routing::post,
};
use nvbes_core::http::error::ErrorEnvelope;

use super::types::MfaRequest;

pub fn router() -> Router<AppState> {
    Router::new().route("/challenge/mfa", post(challenge_mfa))
}

#[utoipa::path(
    post,
    path = "/auth/challenge/mfa",
    tag = "auth",
    request_body = MfaRequest,
    responses(
        (status = 200, description = "Login successful after MFA", body = crate::domains::auth::types::LoginResult),
        (status = 401, description = "Invalid credentials or state", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn challenge_mfa(
    State(state): State<AppState>,
    Query(query): Query<super::LoginQuery>,
    headers: HeaderMap,
    Json(request): Json<MfaRequest>,
) -> Result<Response, AppError> {
    let meta = super::LoginRequestMeta::from_headers(&headers);
    nvbes_core::limiter::check_rate_limit_pair(
        &state.redis,
        "auth_login_mfa",
        nvbes_core::limiter::RateLimitRule {
            key: &meta.rate_limit_ip_key(),
            max_hits: 20,
            window: std::time::Duration::from_secs(60),
        },
        nvbes_core::limiter::RateLimitRule {
            key: &format!("state:{}", request.state_token),
            max_hits: 8,
            window: std::time::Duration::from_secs(300),
        },
    )
    .await?;

    let (auth_state, principal_id) =
        super::require_mfa_state(&state.redis, request.state_token).await?;
    let request_ip = meta.ip();
    let request_user_agent = meta.user_agent();

    let authenticated_method = match super::mfa_flow::resolve_authenticated_method(
        &state.db,
        &state.redis,
        &state.config,
        &request,
        auth_state.id,
        principal_id,
        request_ip.as_deref(),
        request_user_agent.as_deref(),
    )
    .await
    {
        Ok(method) => method,
        Err(err) => {
            let audit_ip = meta.ip();
            let audit_user_agent = meta.user_agent();
            let error_code = err.code.clone();
            let geo_signal = crate::domains::auth::risk::geo::apply_geo_security_signal(
                &state.db,
                &state.config,
                principal_id,
                audit_ip.as_deref(),
                35.0,
                serde_json::json!({ "reason": error_code }),
                "mfa_failed",
            )
            .await;
            let geo_metadata = geo_signal.metadata.clone();
            let _ = crate::domains::auth::risk::record_event(
                &state.db,
                crate::domains::auth::risk::RiskEventInput {
                    principal_id,
                    session_id: None,
                    device_id: None,
                    event_type: "mfa_failed".to_string(),
                    ip_address: audit_ip.clone(),
                    user_agent: audit_user_agent.clone(),
                    risk_score: geo_signal.score,
                    risk_factors: geo_signal.factors,
                    decision: crate::domains::auth::risk::RiskDecision::StepUp,
                    metadata: serde_json::json!({
                        "state_token": request.state_token,
                        "geo": geo_signal.metadata,
                    }),
                },
            )
            .await;
            let _ = record_auth_event(
                &state.db,
                AuthAuditInput {
                    principal_id,
                    action: "auth.mfa_failed",
                    target_type: "principal",
                    target_id: Some(principal_id),
                    ip: audit_ip.as_deref(),
                    user_agent: audit_user_agent.as_deref(),
                    metadata: serde_json::json!({
                        "state_token": request.state_token,
                        "geo": geo_metadata,
                    }),
                },
            )
            .await;
            return Err(err);
        }
    };

    let mut amr = auth_state.completed_methods;
    amr.push(authenticated_method);
    let result = sessions::create_session_for_principal(
        &state.db,
        &state.redis,
        &state.config,
        principal_id,
        super::login_session_context(
            auth_state.email,
            &meta,
            auth_state.device_fingerprint,
            amr,
            "aal2",
        ),
    )
    .await?;

    let secure_cookie = state.config.environment != "development";
    let authuser = query.authuser()?;
    let session_expires_in = (state.config.auth_session_ttl_hours * 60 * 60).max(0);
    let registration_enrollment_expires_in =
        (state.config.auth_verification_ttl_hours * 60 * 60).max(0);
    let response = super::login_response(
        result,
        authuser,
        secure_cookie,
        session_expires_in,
        registration_enrollment_expires_in,
        &state.config.jwt_secret,
        &state.product_analytics,
        &headers,
    )?;
    delete_state(&state.redis, request.state_token).await?;
    Ok(response)
}
