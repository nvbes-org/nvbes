use crate::domains::auth::jwt::JwtService;
use crate::domains::auth::{
    db as auth_db, jwt::types::TokenClaims, sessions, sessions_mgmt, types,
};
use crate::http::error::AppError;
use crate::http::middleware::jwt::session_refresh::RefreshedCookies;
use axum::http::HeaderMap;
use chrono::{TimeZone, Utc};
use sqlx::PgPool;
use uuid::Uuid;

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
    jwt: &JwtService,
    state: &crate::app::AppState,
    headers: &HeaderMap,
) -> Result<(AccountChooserResult, Vec<RefreshedCookies>), AppError> {
    let mut accounts = Vec::new();
    let mut refreshed_cookies = Vec::new();

    for cookie in crate::http::request::session_cookie_tokens(headers) {
        match active_account_from_cookie(db, redis, jwt, &cookie).await {
            Ok(account) => {
                accounts.push(account);
                continue;
            }
            Err(error)
                if error.status == axum::http::StatusCode::UNAUTHORIZED
                    && error.code == "token_expired" =>
            {
                if let Ok((auth_context, cookies)) =
                    crate::http::middleware::jwt::session_refresh::refresh_expired_session(
                        state,
                        &cookie.authuser,
                        &cookie.token,
                    )
                    .await
                    && let Ok(session_view) =
                        sessions_mgmt::fetch_view(redis, auth_context.session_id, false).await
                {
                    refreshed_cookies.push(cookies);
                    accounts.push(account_entry_from_auth_context(
                        cookie.authuser,
                        auth_context,
                        session_view,
                    ));
                    continue;
                }
            }
            Err(_) => {}
        }

        if let Ok(claims) = jwt.decode_token_ignore_expiry(&cookie.token)
            && let Ok(principal_id) = Uuid::parse_str(&claims.sub)
            && let Ok(user) = auth_db::fetch_user_record(db, principal_id).await
        {
            let mfa_enabled = crate::domains::auth::mfa::has_active_factor(db, principal_id)
                .await
                .unwrap_or(false);
            if let Ok(account) =
                account_entry_from_expired_claims(&cookie.authuser, &claims, user, mfa_enabled)
            {
                accounts.push(account);
            }
        }
    }

    Ok((AccountChooserResult { accounts }, refreshed_cookies))
}

async fn active_account_from_cookie(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    jwt: &JwtService,
    cookie: &crate::http::request::SessionCookieToken,
) -> Result<AccountChooserSession, AppError> {
    let auth_context = sessions::authenticate(db, redis, jwt, &cookie.token).await?;
    let session_view = sessions_mgmt::fetch_view(redis, auth_context.session_id, false).await?;
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

fn account_entry_from_expired_claims(
    authuser: &str,
    claims: &TokenClaims,
    user: auth_db::UserRecord,
    mfa_enabled: bool,
) -> Result<AccountChooserSession, AppError> {
    let session_id = Uuid::parse_str(&claims.sid)
        .map_err(|err| AppError::unauthorized("invalid_session", err.to_string()))?;
    let tenant_id = claims
        .tenant_id
        .as_deref()
        .and_then(|value| Uuid::parse_str(value).ok());
    let organization_id = claims
        .organization_id
        .as_deref()
        .and_then(|value| Uuid::parse_str(value).ok());
    let workspace_id = claims
        .workspace_id
        .as_deref()
        .and_then(|value| Uuid::parse_str(value).ok());
    let created_at = Utc
        .timestamp_opt(claims.iat, 0)
        .single()
        .unwrap_or(user.created_at);
    let expires_at = Utc
        .timestamp_opt(claims.exp, 0)
        .single()
        .unwrap_or(created_at);

    Ok(AccountChooserSession {
        authuser: authuser.to_string(),
        status: AccountChooserSessionStatus::Expired,
        message: Some("Session expirée, veuillez vous reconnecter.".to_string()),
        user: types::UserView {
            id: user.principal_id,
            email: user.email,
            display_name: user.display_name,
            firstname: user.firstname,
            lastname: user.lastname,
            username: user.username,
            birthdate: user.birthdate,
            region: user.region,
            email_verified: user.email_verified_at.is_some(),
            mfa_enabled,
            created_at: user.created_at,
        },
        session: types::SessionView {
            id: session_id,
            tenant_id,
            organization_id,
            workspace_id,
            workspace_region: claims.workspace_region.clone(),
            created_at,
            last_seen_at: created_at,
            expires_at,
            revoked_at: Some(expires_at),
            ip: None,
            user_agent: None,
            current: false,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::{AccountChooserSessionStatus, account_entry_from_expired_claims};
    use crate::domains::auth::{db::UserRecord, jwt::types::TokenClaims};
    use chrono::{TimeZone, Utc};
    use uuid::Uuid;

    #[test]
    fn expired_claims_stay_visible_for_reauthentication() {
        let principal_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();
        let created_at = Utc.with_ymd_and_hms(2026, 6, 9, 10, 0, 0).unwrap();
        let expires_at = Utc.with_ymd_and_hms(2026, 6, 9, 11, 0, 0).unwrap();
        let user = UserRecord {
            principal_id,
            email: "expired@example.test".to_string(),
            display_name: "Expired User".to_string(),
            status: "active".to_string(),
            firstname: Some("Expired".to_string()),
            lastname: Some("User".to_string()),
            username: Some("expired".to_string()),
            birthdate: None,
            region: Some("FR".to_string()),
            password_hash: None,
            email_verified_at: Some(created_at),
            created_at,
        };
        let claims = TokenClaims {
            jti: Uuid::new_v4().to_string(),
            sid: session_id.to_string(),
            sub: principal_id.to_string(),
            workspace_id: Some(Uuid::new_v4().to_string()),
            workspace_region: Some("eu".to_string()),
            tenant_id: Some(Uuid::new_v4().to_string()),
            organization_id: None,
            token_type: "access".to_string(),
            scope: "openid profile email".to_string(),
            authorization_details: Vec::new(),
            acr: Some("aal1".to_string()),
            amr: vec!["pwd".to_string()],
            client_id: Some("drive-web".to_string()),
            auth_time: Some(created_at.timestamp()),
            iss: "nvbes-identity".to_string(),
            aud: "nvbes-identity-api".to_string(),
            exp: expires_at.timestamp(),
            iat: created_at.timestamp(),
            nbf: created_at.timestamp(),
            cnf: None,
            act: None,
        };

        let account = account_entry_from_expired_claims("1", &claims, user, false)
            .expect("expired claim should produce an account chooser entry");

        assert_eq!(account.authuser, "1");
        assert_eq!(account.status, AccountChooserSessionStatus::Expired);
        assert_eq!(
            account.message.as_deref(),
            Some("Session expirée, veuillez vous reconnecter.")
        );
        assert_eq!(account.user.email, "expired@example.test");
        assert_eq!(account.session.id, session_id);
        assert_eq!(account.session.expires_at, expires_at);
    }
}
