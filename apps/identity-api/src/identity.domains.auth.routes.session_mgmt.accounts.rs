use crate::http::error::AppError;
use crate::{app::AppState, domains::auth::account_chooser};
pub use account_chooser::{AccountChooserResult, AccountChooserSession};
use axum::{
    Json,
    extract::{Path, State},
    http::header::SET_COOKIE,
    response::{IntoResponse, Response},
};

pub async fn get_accounts(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Response, AppError> {
    let (result, refreshed_cookies) = account_chooser::list_cookie_accounts(
        &state.db,
        &state.redis,
        &state.jwt,
        &state,
        &headers,
    )
    .await?;
    let mut response = Json(result).into_response();
    for cookies in refreshed_cookies {
        response
            .headers_mut()
            .append(SET_COOKIE, cookies.session_cookie);
        response
            .headers_mut()
            .append(SET_COOKIE, cookies.csrf_cookie);
    }
    Ok(response)
}

pub async fn forget_account_cookie(
    State(state): State<AppState>,
    Path(authuser): Path<String>,
) -> Result<Response, AppError> {
    let secure_cookie = state.config.environment != "development";
    let session_cookie_name =
        crate::http::cookies::auth_cookie_name_with_user("session", &authuser, secure_cookie);
    let csrf_cookie_name =
        crate::http::cookies::auth_cookie_name_with_user("csrf_token", &authuser, secure_cookie);

    let mut response = Json(serde_json::json!({ "success": true })).into_response();
    response.headers_mut().append(
        SET_COOKIE,
        crate::http::cookies::auth_cookie(&session_cookie_name, "", 0, secure_cookie)?,
    );
    response.headers_mut().append(
        SET_COOKIE,
        crate::http::cookies::csrf_cookie(&csrf_cookie_name, "", 0, secure_cookie)?,
    );
    Ok(response)
}
