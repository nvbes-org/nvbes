use crate::http::error::AppError;
use crate::{app::AppState, domains::auth::account_chooser};
pub use account_chooser::{AccountChooserResult, AccountChooserSession};
use axum::{
    Json,
    extract::{Path, State},
    http::{Uri, header::SET_COOKIE},
    response::{IntoResponse, Response},
};

pub async fn get_accounts(
    State(state): State<AppState>,
    uri: Uri,
    headers: axum::http::HeaderMap,
) -> Result<Response, AppError> {
    let current_authuser = resolve_authuser(&uri, &headers);
    let (result, refreshed_cookies) = account_chooser::list_cookie_accounts(
        &state.db,
        &state.redis,
        &state.jwt,
        &state,
        &headers,
        &current_authuser,
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

fn resolve_authuser(uri: &Uri, headers: &axum::http::HeaderMap) -> String {
    uri.query()
        .and_then(|query| {
            url::form_urlencoded::parse(query.as_bytes())
                .find(|(key, _)| key == "authuser")
                .map(|(_, value)| value.into_owned())
        })
        .or_else(|| {
            headers
                .get("X-Auth-User")
                .and_then(|header| header.to_str().ok())
                .map(str::to_string)
        })
        .unwrap_or_else(|| "0".to_string())
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
