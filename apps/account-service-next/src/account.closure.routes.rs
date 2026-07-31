use axum::{Extension, Json, extract::State};

use crate::{
    app::AppState, auth::AuthenticatedPrincipal, error::AppError, privacy_models::SuccessResponse,
};

#[utoipa::path(
    post,
    path = "/api/v1/closure",
    tag = "closure",
    operation_id = "requestAccountClosure",
    security(("identityOAuth2" = ["account:delete"])),
    responses(
        (status = 200, body = SuccessResponse),
        (status = 401, body = crate::error::ErrorEnvelope),
        (status = 403, body = crate::error::ErrorEnvelope),
    )
)]
pub async fn request_closure(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedPrincipal>,
) -> Result<Json<SuccessResponse>, AppError> {
    crate::closure_db::request(&state.db, auth.principal_id).await?;
    Ok(Json(SuccessResponse::ok()))
}
