#![allow(dead_code)]

use axum::{
    extract::State,
    http::{HeaderMap, Uri, request::Parts},
    middleware::Next,
    response::Response,
};
use uuid::Uuid;

use crate::app::AppState;
use crate::domains::auth::jwt::TokenClaims;
use crate::domains::auth::types::StepUpSubject;
use crate::http::error::AppError;

#[path = "identity.http.middleware.jwt.account_access.rs"]
pub(crate) mod account_access;
#[path = "identity.http.middleware.jwt.authuser.rs"]
mod authuser;
#[path = "identity.http.middleware.jwt.session_refresh.rs"]
pub(crate) mod session_refresh;

#[derive(Clone, Debug)]
pub enum CredentialSource {
    BrowserSession,
    OAuthBearer(Box<OAuthBearerCredential>),
}

#[derive(Clone, Debug)]
pub struct OAuthBearerCredential {
    pub client_id: Option<String>,
    pub audience: String,
    pub tenant_id: Option<Uuid>,
    pub organization_id: Option<Uuid>,
    pub workspace_id: Option<Uuid>,
    pub workspace_region: Option<String>,
    pub acr: Option<String>,
    pub amr: Vec<String>,
    pub auth_time: Option<i64>,
    pub cnf_jkt: Option<String>,
    pub cnf_x5t_s256: Option<String>,
}

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
    pub cnf_x5t_s256: Option<String>,
    pub credential_source: CredentialSource,
}

impl AuthContext {
    /// Check if the token has a specific scope
    pub fn has_scope(&self, required_scope: &str) -> bool {
        self.scope.split_whitespace().any(|s| s == required_scope)
    }
}

impl AuthContext {
    fn from_authenticated_session(
        auth: crate::domains::auth::types::AuthContext,
        credential_source: CredentialSource,
    ) -> Self {
        let (cnf_jkt, cnf_x5t_s256) = match &credential_source {
            CredentialSource::BrowserSession => (auth.cnf_jkt.clone(), None),
            CredentialSource::OAuthBearer(credential) => {
                (credential.cnf_jkt.clone(), credential.cnf_x5t_s256.clone())
            }
        };
        let (
            tenant_id,
            organization_id,
            workspace_id,
            workspace_region,
            acr,
            amr,
            auth_time,
            client_id,
        ) = match &credential_source {
            CredentialSource::BrowserSession => (
                auth.tenant_id,
                auth.organization_id,
                auth.workspace_id,
                auth.workspace_region,
                auth.acr,
                auth.amr,
                auth.auth_time.map(|value| value.timestamp()),
                auth.client_id,
            ),
            CredentialSource::OAuthBearer(credential) => (
                credential.tenant_id,
                credential.organization_id,
                credential.workspace_id,
                credential.workspace_region.clone(),
                credential.acr.clone(),
                credential.amr.clone(),
                credential.auth_time,
                credential.client_id.clone(),
            ),
        };

        Self {
            user_id: auth.user_id,
            user_email: auth.user_email,
            display_name: auth.display_name,
            email_verified_at: auth.email_verified_at,
            mfa_enabled: auth.mfa_enabled,
            tenant_id,
            organization_id,
            workspace_id,
            workspace_region,
            token_type: "access".to_string(),
            scope: auth.scope,
            jti: auth.session_id.to_string(),
            session_id: auth.session_id,
            acr,
            amr,
            auth_time,
            client_id,
            cnf_jkt,
            cnf_x5t_s256,
            credential_source,
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
    let auth_context = authenticate_request(&state, &headers, request.uri()).await?;
    enforce_mtls_token_binding(&auth_context, request.extensions())?;
    let authorization = crate::http::request::authorization_access_token(&headers)?;
    crate::http::middleware::dpop::enforce_token_binding(
        auth_context.cnf_jkt.as_deref(),
        authorization
            .as_ref()
            .map(|authorization| authorization.scheme),
        request
            .extensions()
            .get::<crate::http::middleware::dpop::DpopContext>()
            .map(|context| context.jkt.as_str()),
    )?;
    request.extensions_mut().insert(auth_context);

    Ok(next.run(request).await)
}

pub(crate) async fn authenticate_request(
    state: &AppState,
    headers: &HeaderMap,
    uri: &Uri,
) -> Result<AuthContext, AppError> {
    let authuser = authuser::resolve_authuser(uri, headers)?;
    let bearer_token = crate::http::request::authorization_bearer_token(headers)?;
    let credential_source = bearer_token
        .as_deref()
        .map(|token| state.jwt.decode_token(token, "access"))
        .transpose()?
        .map(OAuthBearerCredential::from_claims)
        .transpose()?
        .map(Box::new)
        .map(CredentialSource::OAuthBearer)
        .unwrap_or(CredentialSource::BrowserSession);
    let auth = session_refresh::authenticate_session_request(state, headers, &authuser).await?;

    Ok(AuthContext::from_authenticated_session(
        auth,
        credential_source,
    ))
}

impl OAuthBearerCredential {
    fn from_claims(claims: TokenClaims) -> Result<Self, AppError> {
        let cnf_jkt = claims.cnf.as_ref().and_then(|cnf| cnf.jkt.clone());
        let cnf_x5t_s256 = claims.cnf.as_ref().and_then(|cnf| cnf.x5t_s256.clone());
        Ok(Self {
            client_id: claims
                .client_id
                .map(|client_id| client_id.trim().to_string())
                .filter(|client_id| !client_id.is_empty()),
            audience: claims.aud,
            tenant_id: parse_optional_uuid_claim(claims.tenant_id, "tenant_id")?,
            organization_id: parse_optional_uuid_claim(claims.organization_id, "organization_id")?,
            workspace_id: parse_optional_uuid_claim(claims.workspace_id, "workspace_id")?,
            workspace_region: claims.workspace_region,
            acr: claims.acr,
            amr: claims.amr,
            auth_time: claims.auth_time,
            cnf_jkt,
            cnf_x5t_s256,
        })
    }
}

fn enforce_mtls_token_binding(
    auth: &AuthContext,
    extensions: &axum::http::Extensions,
) -> Result<(), AppError> {
    let Some(expected) = auth.cnf_x5t_s256.as_deref() else {
        return Ok(());
    };
    let actual = extensions
        .get::<crate::http::mtls::MtlsCertificateThumbprint>()
        .map(|thumbprint| thumbprint.0.as_str());
    if actual == Some(expected) {
        Ok(())
    } else {
        Err(AppError::unauthorized(
            "mtls_certificate_mismatch",
            "The access token is bound to a different mutual-TLS certificate.",
        ))
    }
}

fn parse_optional_uuid_claim(
    value: Option<String>,
    claim_name: &'static str,
) -> Result<Option<Uuid>, AppError> {
    value
        .map(|value| {
            Uuid::parse_str(&value).map_err(|_| {
                AppError::unauthorized(
                    "invalid_token_context",
                    format!("The access token contains an invalid {claim_name} claim."),
                )
            })
        })
        .transpose()
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

#[cfg(test)]
#[path = "identity.http.middleware.jwt.tests.rs"]
mod tests;
