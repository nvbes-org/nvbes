use crate::domains::auth::jwt::JwtService;
use crate::domains::auth::{sessions, sessions_mgmt};
use crate::http::error::AppError;
use crate::http::middleware::jwt::AuthContext;
use axum::http::HeaderMap;
use sqlx::PgPool;
use std::collections::HashSet;

pub async fn logout_browser_sessions(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    jwt: &JwtService,
    auth: &AuthContext,
    headers: &HeaderMap,
) -> Result<(), AppError> {
    let mut seen = HashSet::from([(auth.session_id, auth.user_id)]);
    sessions_mgmt::logout(db, redis, auth.session_id, auth.user_id).await?;

    for cookie in crate::http::request::session_cookie_tokens(headers) {
        if let Ok(cookie_auth) = sessions::authenticate(db, redis, jwt, &cookie.token).await {
            let key = (cookie_auth.session_id, cookie_auth.user_id);
            if seen.insert(key) {
                sessions_mgmt::logout(db, redis, cookie_auth.session_id, cookie_auth.user_id)
                    .await?;
            }
        }
    }

    Ok(())
}
