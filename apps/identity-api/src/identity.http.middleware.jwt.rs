#![allow(dead_code)]

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode, header::SET_COOKIE, request::Parts},
    middleware::Next,
    response::Response,
};
use uuid::Uuid;

use crate::app::AppState;
use crate::domains::auth::sessions;
use crate::domains::auth::types::StepUpSubject;
use crate::http::cookies::{
    auth_cookie, auth_cookie_name_with_user, csrf_cookie, generate_csrf_token,
};
use crate::http::error::AppError;

/// Extracted authentication context from JWT
#[derive(Clone, Debug)]
pub struct AuthContext {
    pub user_id: Uuid,
    pub user_email: String,
    pub display_name: String,
    pub email_verified_at: Option<chrono::DateTime<chrono::Utc>>,
    pub mfa_enabled: bool,
    pub tenant_id: Option<Uuid>,
    pub organization_id: Option<Uuid>,
    pub workspace_id: Option<Uuid>,
    pub workspace_region: Option<String>,
    pub token_type: String,
    pub scope: String,
    pub jti: String,
    pub session_id: Uuid,
    pub acr: Option<String>,
    pub amr: Vec<String>,
    pub auth_time: Option<i64>,
    pub client_id: Option<String>,
    pub cnf_jkt: Option<String>,
}

impl AuthContext {
    /// Check if the token has a specific scope
    pub fn has_scope(&self, required_scope: &str) -> bool {
        self.scope.split_whitespace().any(|s| s == required_scope)
    }
}

impl StepUpSubject for AuthContext {
    fn user_id(&self) -> Uuid {
        self.user_id
    }

    fn session_id(&self) -> Uuid {
        self.session_id
    }

    fn tenant_id(&self) -> Option<Uuid> {
        self.tenant_id
    }

    fn workspace_id(&self) -> Option<Uuid> {
        self.workspace_id
    }
}

/// Middleware to validate JWT tokens
pub async fn jwt_auth_middleware(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut request: axum::http::Request<axum::body::Body>,
    next: Next,
) -> Result<Response, AppError> {
    let authuser = request
        .uri()
        .query()
        .and_then(|q| {
            url::form_urlencoded::parse(q.as_bytes())
                .find(|(k, _)| k == "authuser")
                .map(|(_, v)| v.into_owned())
        })
        .or_else(|| {
            headers
                .get("X-Auth-User")
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "0".to_string());

    let token = crate::http::request::bearer_token_with_authuser(&headers, &authuser)?;

    let (auth, refresh_cookie) =
        match sessions::authenticate(&state.db, &state.redis, &state.jwt, &token).await {
            Ok(auth) => (auth, None),
            Err(e) if e.status == StatusCode::UNAUTHORIZED && e.code == "token_expired" => {
                let expired_token = token.clone();
                let claims = state.jwt.decode_token_ignore_expiry(&expired_token)?;

                let session_id = Uuid::parse_str(&claims.sid)
                    .map_err(|_| AppError::unauthorized("invalid_session", "Invalid session ID"))?;
                let principal_id = Uuid::parse_str(&claims.sub)
                    .map_err(|_| AppError::unauthorized("invalid_subject", "Invalid subject"))?;

                let session =
                    nvbes_redis::session::get_session(&state.redis, &session_id.to_string())
                        .await
                        .map_err(|err| {
                            AppError::internal("redis_session_read_failed", &err.to_string())
                        })?
                        .ok_or_else(|| {
                            AppError::unauthorized("session_not_found", "Session not found")
                        })?;

                if session.principal_id != principal_id.to_string()
                    || session.revoked_at.is_some()
                    || session.expires_at <= chrono::Utc::now()
                {
                    return Err(AppError::unauthorized("session_expired", "Session expired"));
                }

                let new_token = state.jwt.generate_access_token_from_claims(&claims)?;

                let session_ttl = (session.expires_at - chrono::Utc::now())
                    .num_seconds()
                    .max(1) as u64;
                if nvbes_redis::session::update_session_token_hash(
                    &state.redis,
                    &session_id.to_string(),
                    &crate::domains::auth::password::token_hash(&new_token),
                    session_ttl,
                )
                .await
                .is_err()
                {
                    nvbes_redis::session::delete_session(
                        &state.redis,
                        &principal_id.to_string(),
                        &session_id.to_string(),
                    )
                    .await
                    .map_err(|err| {
                        AppError::internal("redis_session_cache_reset_failed", &err.to_string())
                    })?;
                }

                let auth =
                    sessions::authenticate(&state.db, &state.redis, &state.jwt, &new_token).await?;

                let secure_cookie = state.config.environment != "development";
                let session_cookie_name =
                    auth_cookie_name_with_user("session", &authuser, secure_cookie);
                let session_expires_in = (state.config.auth_session_ttl_hours * 60 * 60).max(0);
                let cookie = auth_cookie(
                    &session_cookie_name,
                    &new_token,
                    session_expires_in,
                    secure_cookie,
                )?;
                let csrf_token = generate_csrf_token();
                let csrf_cookie_name =
                    auth_cookie_name_with_user("csrf_token", &authuser, secure_cookie);
                let csrf_cookie_value = csrf_cookie(
                    &csrf_cookie_name,
                    &csrf_token,
                    session_expires_in,
                    secure_cookie,
                )?;

                (auth, Some((cookie, csrf_cookie_value)))
            }
            Err(e) => return Err(e),
        };

    // Create auth context
    let cnf_jkt = auth.cnf_jkt.clone();

    let auth_context = AuthContext {
        user_id: auth.user_id,
        user_email: auth.user_email,
        display_name: auth.display_name,
        email_verified_at: auth.email_verified_at,
        mfa_enabled: auth.mfa_enabled,
        tenant_id: auth.tenant_id,
        organization_id: auth.organization_id,
        workspace_id: auth.workspace_id,
        workspace_region: auth.workspace_region,
        token_type: "access".to_string(),
        scope: auth.scope,
        jti: auth.session_id.to_string(),
        session_id: auth.session_id,
        acr: auth.acr,
        amr: auth.amr,
        auth_time: auth.auth_time.map(|value| value.timestamp()),
        client_id: auth.client_id,
        cnf_jkt,
    };

    // Store auth context in request extensions
    request.extensions_mut().insert(auth_context);

    let mut response = next.run(request).await;

    // Set refreshed cookies on response if token was rotated
    if let Some((session_cookie, csrf_cookie)) = refresh_cookie {
        response.headers_mut().append(SET_COOKIE, session_cookie);
        response.headers_mut().append(SET_COOKIE, csrf_cookie);
    }

    Ok(response)
}

/// Extension trait to extract AuthContext from request
pub trait AuthContextExtractor {
    fn auth_context(&self) -> Result<&AuthContext, AppError>;
}

impl AuthContextExtractor for Parts {
    fn auth_context(&self) -> Result<&AuthContext, AppError> {
        self.extensions.get::<AuthContext>().ok_or_else(|| {
            AppError::unauthorized("no_auth_context", "No authentication context found")
        })
    }
}
