use crate::domains::auth::jwt::JwtService;
use crate::domains::auth::{sessions, sessions_mgmt, types};
use crate::http::error::AppError;
use axum::http::HeaderMap;
use sqlx::PgPool;

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct AccountChooserSession {
    pub authuser: String,
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
    jwt: &JwtService,
    headers: &HeaderMap,
) -> Result<AccountChooserResult, AppError> {
    let mut accounts = Vec::new();

    for cookie in crate::http::request::session_cookie_tokens(headers) {
        if let Ok(auth_context) = sessions::authenticate(db, redis, jwt, &cookie.token).await
            && let Ok(session_view) =
                sessions_mgmt::fetch_view(redis, auth_context.session_id, false).await
        {
            accounts.push(AccountChooserSession {
                authuser: cookie.authuser,
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
                    created_at: chrono::Utc::now(),
                },
                session: session_view,
            });
        }
    }

    Ok(AccountChooserResult { accounts })
}
