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
    let current_authuser = crate::http::authuser::from_uri_and_headers(&uri, &headers)?;
    let result =
        account_chooser::list_cookie_accounts(&state.db, &state.redis, &headers, &current_authuser)
            .await?;
    Ok(Json(result).into_response())
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
