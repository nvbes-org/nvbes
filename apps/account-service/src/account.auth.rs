use crate::{app::AccountState, error::AccountError};
use axum::{extract::FromRequestParts, http::request::Parts};
use uuid::Uuid;

#[path = "account.auth.tokens.rs"]
mod tokens;
pub use tokens::TokenVerifier;

#[derive(Debug, Clone)]
pub struct Principal {
    pub id: Uuid,
    scopes: Vec<String>,
    strong_until: Option<u64>,
}
impl Principal {
    pub fn require(&self, scope: &str) -> Result<(), AccountError> {
        self.scopes
            .iter()
            .any(|candidate| candidate == scope)
            .then_some(())
            .ok_or(AccountError::Forbidden)
    }
    pub fn require_step_up(&self) -> Result<(), AccountError> {
        self.strong_until
            .is_some_and(|until| until > chrono::Utc::now().timestamp() as u64)
            .then_some(())
            .ok_or(AccountError::Forbidden)
    }
}
impl FromRequestParts<AccountState> for Principal {
    type Rejection = AccountError;
    async fn from_request_parts(
        parts: &mut Parts,
        state: &AccountState,
    ) -> Result<Self, Self::Rejection> {
        let mut headers = parts.headers.get_all("authorization").iter();
        let value = headers.next().ok_or(AccountError::Unauthorized)?;
        if headers.next().is_some() {
            return Err(AccountError::Unauthorized);
        }
        let token = value
            .to_str()
            .ok()
            .and_then(|v| v.strip_prefix("Bearer "))
            .filter(|v| !v.is_empty() && !v.bytes().any(|b| b.is_ascii_whitespace()))
            .ok_or(AccountError::Unauthorized)?;
        state.tokens.authenticate(token).await
    }
}
