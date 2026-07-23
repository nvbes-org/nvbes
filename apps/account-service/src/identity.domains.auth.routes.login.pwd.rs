use crate::app::AppState;
use crate::domains::auth::{
    audit::{AuthAuditInput, record_auth_event},
    exposed_credentials::{self, ExposedCredentialCheck},
    mfa,
    risk::{self, RiskDecision, RiskEventInput},
    sessions,
    state::{create_state, delete_state, fetch_state},
};
use crate::http::error::AppError;
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
};
use nvbes_core::http::error::ErrorEnvelope;

use super::types::{IdentifierResult, PwdRequest};

pub fn router() -> Router<AppState> {
    Router::new().route("/challenge/pwd", post(challenge_pwd))
}

#[utoipa::path(
    post,
    path = "/auth/challenge/pwd",
    tag = "auth",
    request_body = PwdRequest,
    responses(
        (status = 200, description = "Login successful", body = crate::domains::auth::types::LoginResult),
        (status = 202, description = "Password accepted, MFA challenge required", body = IdentifierResult),
        (status = 403, description = "Password is compromised", body = ErrorEnvelope),
        (status = 401, description = "Invalid credentials or state", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn challenge_pwd(
    State(state): State<AppState>,
    axum::extract::Query(query): axum::extract::Query<super::LoginQuery>,
    headers: HeaderMap,
    Json(request): Json<PwdRequest>,
) -> Result<Response, AppError> {
    let meta = super::LoginRequestMeta::from_headers(&headers);
    let auth_state = fetch_state(&state.redis, request.state_token, "pwd").await?;
    crate::domains::auth::sso_policy::ensure_password_allowed_for_email(
        &state.db,
        &auth_state.email,
    )
    .await?;
    crate::domains::auth::check_rate_limit(
        &state.redis,
        "auth_login",
        &format!("key:{}", auth_state.email),
        10,
        std::time::Duration::from_secs(60),
    )
    .await?;

    let login_input = crate::domains::auth::types::LoginInput {
        email: auth_state.email.clone(),
        password: request.password,
        ip: meta.ip(),
        user_agent: meta.user_agent(),
        device_fingerprint: auth_state.device_fingerprint.clone(),
    };
    let mut verified =
        sessions::verify_primary_credentials(&state.db, &state.redis, &state.config, &login_input)
            .await?;
    verified.risk_score += crate::domains::auth::device_trust::pre_auth_risk_score(
        &state.db,
        verified.principal_id,
        meta.installation_token(),
        auth_state.device_fingerprint.as_ref(),
        &state.config.jwt_secret,
    )
    .await?;
    if let Some(check) =
        ExposedCredentialCheck::from_headers(&headers).filter(|check| check.password_leaked())
    {
        record_exposed_login_password(&state, verified.principal_id, &meta, check).await;
        delete_state(&state.redis, request.state_token).await?;
        return Err(exposed_credentials::login_rejected_error());
    }

    if let Some(available_methods) = super::identifier_flow::resolve_post_password_challenge(
        &state.db,
        verified.principal_id,
        verified.risk_score,
    )
    .await?
    {
        let mfa_state_token = create_state(
            &state.redis,
            Some(verified.principal_id),
            &auth_state.email,
            "mfa",
            auth_state.device_fingerprint,
        )
        .await?;

        if available_methods.iter().any(|method| method == "email") {
            mfa::email::send_login_code(
                &state.db,
                &state.redis,
                mfa_state_token,
                verified.principal_id,
            )
            .await?;
        }

        let response = (
            StatusCode::ACCEPTED,
            Json(IdentifierResult {
                next_step: "mfa".to_string(),
                state_token: mfa_state_token,
                available_methods: Some(available_methods),
            }),
        )
            .into_response();

        delete_state(&state.redis, request.state_token).await?;
        return Ok(response);
    }

    let result = sessions::create_session_for_principal(
        &state.db,
        &state.redis,
        &state.config,
        verified.principal_id,
        super::login_session_context(
            verified.email,
            &meta,
            auth_state.device_fingerprint,
            vec!["pwd".to_string()],
            "aal1",
        ),
    )
    .await?;

    let secure_cookie = state.config.environment != "development";
    let authuser = query.authuser()?;
    let session_expires_in = (state.config.auth_session_ttl_hours * 60 * 60).max(0);
    let response = super::login_response(
        result,
        authuser,
        secure_cookie,
        session_expires_in,
        &state.config.jwt_secret,
    )?;
    delete_state(&state.redis, request.state_token).await?;
    Ok(response)
}

async fn record_exposed_login_password(
    state: &AppState,
    principal_id: uuid::Uuid,
    meta: &super::LoginRequestMeta,
    check: ExposedCredentialCheck,
) {
    let labels = check.labels();
    let ip = meta.ip();
    let user_agent = meta.user_agent();
    let _ = risk::record_event(
        &state.db,
        RiskEventInput {
            principal_id,
            session_id: None,
            device_id: None,
            event_type: "login_compromised_password".to_string(),
            ip_address: ip.clone(),
            user_agent: user_agent.clone(),
            risk_score: 65.0,
            risk_factors: serde_json::json!({
                "leaked_credentials": labels.clone(),
            }),
            decision: RiskDecision::Deny,
            metadata: serde_json::json!({
                "source": "cloudflare_exposed_credential_check",
            }),
        },
    )
    .await;
    let _ = record_auth_event(
        &state.db,
        AuthAuditInput {
            principal_id,
            action: "auth.login_compromised_password",
            target_type: "principal",
            target_id: Some(principal_id),
            ip: ip.as_deref(),
            user_agent: user_agent.as_deref(),
            metadata: serde_json::json!({
                "source": "cloudflare_exposed_credential_check",
                "leaked_credentials": labels,
            }),
        },
    )
    .await;
}
