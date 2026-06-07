use crate::http::error::AppError;
use crate::{app::AppState, domains::auth::account_chooser};
pub use account_chooser::{AccountChooserResult, AccountChooserSession};
use axum::{
    Json,
    extract::State,
    response::{IntoResponse, Response},
};

pub async fn get_accounts(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Response, AppError> {
    let result =
        account_chooser::list_cookie_accounts(&state.db, &state.redis, &state.jwt, &headers)
            .await?;
    Ok(Json(result).into_response())
}
