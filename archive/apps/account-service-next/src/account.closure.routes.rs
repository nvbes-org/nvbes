use axum::{Extension, Json, extract::State, http::StatusCode};

use crate::{app::AppState, auth::AuthenticatedPrincipal, error::AppError};

#[utoipa::path(
    post,
    path = "/api/v1/closure",
    tag = "closure",
    operation_id = "requestAccountClosure",
    security(("identityOAuth2" = ["account:delete"])),
    responses(
        (status = 202, body = crate::closure_db::ClosureRequest),
        (status = 401, body = crate::error::ErrorEnvelope),
        (status = 403, body = crate::error::ErrorEnvelope),
    )
)]
pub async fn request_closure(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedPrincipal>,
) -> Result<(StatusCode, Json<crate::closure_db::ClosureRequest>), AppError> {
    let closure = crate::closure_db::request(&state.db, auth.principal_id).await?;
    Ok((StatusCode::ACCEPTED, Json(closure)))
}

#[utoipa::path(
    get,
    path = "/api/v1/closure",
    tag = "closure",
    operation_id = "getAccountClosure",
    security(("identityOAuth2" = ["account:delete"])),
    responses(
        (status = 200, body = crate::closure_db::ClosureStatus),
        (status = 401, body = crate::error::ErrorEnvelope),
        (status = 403, body = crate::error::ErrorEnvelope),
        (status = 404, body = crate::error::ErrorEnvelope),
    )
)]
pub async fn get_closure(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedPrincipal>,
) -> Result<Json<crate::closure_db::ClosureStatus>, AppError> {
    Ok(Json(
        crate::closure_db::latest(&state.db, auth.principal_id).await?,
    ))
}
