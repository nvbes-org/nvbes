use crate::{app::BillingState, error::BillingError};
use axum::{extract::FromRequestParts, http::request::Parts};
use uuid::Uuid;

#[path = "billing.auth.tokens.rs"]
mod tokens;
pub use tokens::TokenVerifier;

#[derive(Debug, Clone)]
pub struct BillingPrincipal {
    id: Uuid,
    scopes: Vec<String>,
    strong_authentication: bool,
}
impl BillingPrincipal {
    pub fn id(&self) -> Uuid {
        self.id
    }
    pub fn has_mfa(&self) -> bool {
        self.strong_authentication
    }
    pub fn require_scope(&self, scope: &str) -> Result<(), BillingError> {
        if self.scopes.iter().any(|s| s == scope) {
            Ok(())
        } else {
            Err(BillingError::Forbidden)
        }
    }
}

fn bearer(parts: &Parts) -> Result<&str, BillingError> {
    let mut values = parts.headers.get_all("authorization").iter();
    let value = values.next().ok_or(BillingError::Unauthorized)?;
    if values.next().is_some() {
        return Err(BillingError::Unauthorized);
    }
    value
        .to_str()
        .ok()
        .and_then(|v| v.strip_prefix("Bearer "))
        .filter(|v| !v.is_empty() && !v.bytes().any(|b| b.is_ascii_whitespace()))
        .ok_or(BillingError::Unauthorized)
}
impl FromRequestParts<BillingState> for BillingPrincipal {
    type Rejection = BillingError;
    async fn from_request_parts(
        parts: &mut Parts,
        state: &BillingState,
    ) -> Result<Self, Self::Rejection> {
        let credentials = nvbes_dpop::resource::Credentials::from_headers(&parts.headers)
            .map_err(|_| BillingError::Unauthorized)?;
        let uri = parts
            .extensions
            .get::<axum::extract::OriginalUri>()
            .map(|original| &original.0)
            .unwrap_or(&parts.uri);
        state
            .tokens
            .authenticate(&credentials, &parts.method, uri, &state.db)
            .await
    }
}

pub struct OperatorAuth;
impl FromRequestParts<BillingState> for OperatorAuth {
    type Rejection = BillingError;
    async fn from_request_parts(
        parts: &mut Parts,
        state: &BillingState,
    ) -> Result<Self, Self::Rejection> {
        let token = bearer(parts)?;
        if let Some(expected) = &state.config.operator_token
            && nvbes_billing::stripe::constant_time_eq(token.as_bytes(), expected.as_bytes())
        {
            return Ok(OperatorAuth);
        }
        Err(BillingError::Unauthorized)
    }
}
