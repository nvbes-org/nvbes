use crate::domains::auth::{sessions, sessions_mgmt, types};
use crate::http::error::AppError;
use axum::http::HeaderMap;
use chrono::Utc;
use sqlx::PgPool;

#[derive(Debug, PartialEq, Eq, serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AccountChooserSessionStatus {
    Active,
    Expired,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct AccountChooserSession {
    pub authuser: String,
    pub status: AccountChooserSessionStatus,
    pub message: Option<String>,
    pub user: types::UserView,
    pub session: types::SessionView,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct AccountChooserResult {
    pub accounts: Vec<AccountChooserSession>,
}

pub async fn list_cookie_accounts(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    headers: &HeaderMap,
    current_authuser: &str,
) -> Result<AccountChooserResult, AppError> {
    let mut accounts = Vec::new();

    for cookie in crate::http::request::session_cookie_tokens(headers) {
        let current = cookie.authuser == current_authuser;
        if let Ok(account) = active_account_from_cookie(db, redis, headers, &cookie, current).await
        {
            accounts.push(account);
        }
    }

    Ok(AccountChooserResult { accounts })
}

async fn active_account_from_cookie(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    headers: &HeaderMap,
    cookie: &crate::http::request::SessionCookieToken,
    current: bool,
) -> Result<AccountChooserSession, AppError> {
    let auth_context =
        sessions::authenticate_browser_session(db, redis, &cookie.token, headers).await?;
    let session_view = sessions_mgmt::fetch_view(redis, auth_context.session_id, current).await?;
    Ok(account_entry_from_auth_context(
        cookie.authuser.clone(),
        auth_context,
        session_view,
    ))
}

fn account_entry_from_auth_context(
    authuser: String,
    auth_context: types::AuthContext,
    session: types::SessionView,
) -> AccountChooserSession {
    AccountChooserSession {
        authuser,
        status: AccountChooserSessionStatus::Active,
        message: None,
        user: types::UserView {
            id: auth_context.user_id,
            email: auth_context.user_email,
            display_name: auth_context.display_name,
            firstname: None,
            lastname: None,
            username: None,
            birthdate: None,
            region: None,
            email_verified: auth_context.email_verified_at.is_some(),
            mfa_enabled: auth_context.mfa_enabled,
            created_at: Utc::now(),
        },
        session,
    }
}

#[cfg(test)]
#[path = "identity.domains.auth.account_chooser.tests.rs"]
mod tests;
