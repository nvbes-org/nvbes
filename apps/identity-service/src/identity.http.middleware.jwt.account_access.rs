use axum::{
    extract::State, http::HeaderMap, middleware::Next, response::Response, routing::MethodRouter,
};

use crate::app::AppState;
use crate::http::error::AppError;

use super::{AuthContext, CredentialSource, authenticate_request};

pub const PROFILE_READ_SCOPE: &str = "account:profile:read";
pub const PROFILE_WRITE_SCOPE: &str = "account:profile:write";
pub const EMAIL_READ_SCOPE: &str = "account:email:read";
pub const EMAIL_WRITE_SCOPE: &str = "account:email:write";
pub const SESSION_READ_SCOPE: &str = "account:session:read";
pub const SESSION_WRITE_SCOPE: &str = "account:session:write";
pub const SECURITY_READ_SCOPE: &str = "account:security:read";
pub const SECURITY_WRITE_SCOPE: &str = "account:security:write";
pub const PREFERENCES_READ_SCOPE: &str = "account:preferences:read";
pub const PREFERENCES_WRITE_SCOPE: &str = "account:preferences:write";
pub const EXPORT_SCOPE: &str = "account:export";
pub const DELETE_SCOPE: &str = "account:delete";
pub const LEGAL_READ_SCOPE: &str = "account:legal:read";
pub const LEGAL_WRITE_SCOPE: &str = "account:legal:write";
pub const OAUTH_CLIENTS_READ_SCOPE: &str = "account:oauth-clients:read";
pub const OAUTH_CLIENTS_WRITE_SCOPE: &str = "account:oauth-clients:write";
pub const OAUTH_APPROVAL_SCOPE: &str = "account:oauth:approve";

pub const SUPPORTED_ACCOUNT_SCOPES: &[&str] = &[
    PROFILE_READ_SCOPE,
    PROFILE_WRITE_SCOPE,
    EMAIL_READ_SCOPE,
    EMAIL_WRITE_SCOPE,
    SESSION_READ_SCOPE,
    SESSION_WRITE_SCOPE,
    SECURITY_READ_SCOPE,
    SECURITY_WRITE_SCOPE,
    PREFERENCES_READ_SCOPE,
    PREFERENCES_WRITE_SCOPE,
    EXPORT_SCOPE,
    DELETE_SCOPE,
    LEGAL_READ_SCOPE,
    LEGAL_WRITE_SCOPE,
    OAUTH_CLIENTS_READ_SCOPE,
    OAUTH_CLIENTS_WRITE_SCOPE,
    OAUTH_APPROVAL_SCOPE,
];

#[derive(Clone, Copy, Debug)]
pub enum AccountAccess {
    BrowserSession,
    OAuthScope(&'static str),
}

#[derive(Clone)]
struct AccountAccessState {
    app: AppState,
    access: AccountAccess,
}

#[derive(Debug)]
struct OAuthPolicyCheck {
    client_id: String,
    tenant_id: Option<uuid::Uuid>,
    organization_id: Option<uuid::Uuid>,
    workspace_id: Option<uuid::Uuid>,
    required_scope: &'static str,
}

pub fn protected_method(
    state: &AppState,
    access: AccountAccess,
    method: MethodRouter<AppState>,
) -> MethodRouter<AppState> {
    method.layer(axum::middleware::from_fn_with_state(
        AccountAccessState {
            app: state.clone(),
            access,
        },
        account_access_middleware,
    ))
}

async fn account_access_middleware(
    State(state): State<AccountAccessState>,
    headers: HeaderMap,
    mut request: axum::http::Request<axum::body::Body>,
    next: Next,
) -> Result<Response, AppError> {
    let auth = authenticate_request(&state.app, &headers, request.uri()).await?;
    if let Some(check) = validate_account_access(&auth, state.access, &state.app.jwt.audience)? {
        crate::domains::oauth::policies_eval::ensure_client_policy(
            &state.app.db,
            &check.client_id,
            check.tenant_id,
            check.organization_id,
            check.workspace_id,
            check.required_scope,
            Some(&state.app.jwt.audience),
            &[],
        )
        .await?;
    }

    request.extensions_mut().insert(auth);
    Ok(next.run(request).await)
}

fn validate_account_access(
    auth: &AuthContext,
    access: AccountAccess,
    account_audience: &str,
) -> Result<Option<OAuthPolicyCheck>, AppError> {
    let (credential, required_scope) = match (&auth.credential_source, access) {
        (CredentialSource::BrowserSession, AccountAccess::BrowserSession) => return Ok(None),
        (CredentialSource::BrowserSession, AccountAccess::OAuthScope(_)) => {
            return Err(AppError::unauthorized(
                "account_bearer_token_required",
                "This Account API operation requires an OAuth access token.",
            ));
        }
        (CredentialSource::OAuthBearer(_), AccountAccess::BrowserSession) => {
            return Err(AppError::forbidden(
                "browser_session_required",
                "This Identity operation requires a first-party browser session.",
            ));
        }
        (CredentialSource::OAuthBearer(credential), AccountAccess::OAuthScope(required_scope)) => {
            (credential, required_scope)
        }
    };

    if credential.audience != account_audience {
        return Err(AppError::forbidden(
            "account_token_audience_invalid",
            "The OAuth access token is not intended for the Account service.",
        ));
    }

    let client_id = credential.client_id.clone().ok_or_else(|| {
        AppError::forbidden(
            "account_oauth_client_required",
            "The OAuth access token is missing its client identifier.",
        )
    })?;

    if !auth.has_scope(required_scope) {
        return Err(AppError::forbidden(
            "insufficient_scope",
            format!("The OAuth access token requires the {required_scope} scope."),
        ));
    }

    Ok(Some(OAuthPolicyCheck {
        client_id,
        tenant_id: credential.tenant_id,
        organization_id: credential.organization_id,
        workspace_id: credential.workspace_id,
        required_scope,
    }))
}

#[cfg(test)]
#[path = "identity.http.middleware.jwt.account_access.tests.rs"]
mod tests;
