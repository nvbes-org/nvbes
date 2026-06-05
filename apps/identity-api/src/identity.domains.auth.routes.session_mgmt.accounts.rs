use crate::domains::auth::sessions_mgmt;
use crate::domains::auth::types;
use crate::http::error::AppError;
use crate::{app::AppState, domains::auth::sessions};
use axum::{
    Json,
    extract::State,
    response::{IntoResponse, Response},
};

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

pub async fn get_accounts(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Response, AppError> {
    let mut active_accounts = vec![];

    if let Some(cookie_header) = headers.get("Cookie") {
        if let Ok(cookie_str) = cookie_header.to_str() {
            for cookie in cookie_str.split(';') {
                let cookie = cookie.trim();
                let parts: Vec<&str> = cookie.splitn(2, '=').collect();
                if parts.len() != 2 {
                    continue;
                }

                let name = parts[0];
                let token = parts[1];
                let authuser = if let Some(suffix) = name.strip_prefix("__Host-session_") {
                    Some(suffix)
                } else if let Some(suffix) = name.strip_prefix("session_") {
                    Some(suffix)
                } else if name == "__Host-session" || name == "session" {
                    Some("0")
                } else {
                    None
                };

                let Some(authuser) = authuser else {
                    continue;
                };

                if let Ok(auth_context) =
                    sessions::authenticate(&state.db, &state.redis, &state.jwt, token).await
                    && let Ok(session_view) =
                        sessions_mgmt::fetch_view(&state.redis, auth_context.session_id, false)
                            .await
                {
                    active_accounts.push(AccountChooserSession {
                        authuser: authuser.to_string(),
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
        }
    }

    Ok(Json(AccountChooserResult {
        accounts: active_accounts,
    })
    .into_response())
}
