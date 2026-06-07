#![allow(dead_code)]

use axum::{
    extract::State,
    http::{HeaderMap, header::SET_COOKIE, request::Parts},
    middleware::Next,
    response::Response,
};
use uuid::Uuid;

use crate::app::AppState;
use crate::domains::auth::types::StepUpSubject;
use crate::http::error::AppError;

#[path = "identity.http.middleware.jwt.authuser.rs"]
mod authuser;
#[path = "identity.http.middleware.jwt.session_refresh.rs"]
mod session_refresh;

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

impl From<crate::domains::auth::types::AuthContext> for AuthContext {
    fn from(auth: crate::domains::auth::types::AuthContext) -> Self {
        let cnf_jkt = auth.cnf_jkt.clone();

        Self {
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
        }
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
    let authuser = authuser::resolve_authuser(request.uri(), &headers);
    let (auth, refreshed_cookies) =
        session_refresh::authenticate_or_refresh_session(&state, &headers, &authuser).await?;
    let auth_context = AuthContext::from(auth);

    // Store auth context in request extensions
    request.extensions_mut().insert(auth_context);

    let mut response = next.run(request).await;

    // Set refreshed cookies on response if token was rotated
    if let Some(refreshed_cookies) = refreshed_cookies {
        response
            .headers_mut()
            .append(SET_COOKIE, refreshed_cookies.session_cookie);
        response
            .headers_mut()
            .append(SET_COOKIE, refreshed_cookies.csrf_cookie);
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
