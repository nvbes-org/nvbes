use std::time::Duration;

use axum::{
    Extension, Json,
    extract::{State, rejection::JsonRejection},
    http::{StatusCode, header},
    response::Response,
};

use crate::{
    app::AppState,
    auth::AuthenticatedPrincipal,
    avatar_models::{AvatarUploadInput, AvatarUploadResponse},
    error::AppError,
    privacy_models::SuccessResponse,
};

#[utoipa::path(
    post,
    path = "/api/v1/profile/avatar",
    tag = "profile",
    operation_id = "prepareAccountAvatarUpload",
    security(("identityOAuth2" = ["account:profile:write"])),
    request_body = AvatarUploadInput,
    responses(
        (status = 200, body = AvatarUploadResponse),
        (status = 400, body = crate::error::ErrorEnvelope),
        (status = 401, body = crate::error::ErrorEnvelope),
        (status = 403, body = crate::error::ErrorEnvelope),
    )
)]
pub async fn prepare_avatar_upload(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedPrincipal>,
    payload: Result<Json<AvatarUploadInput>, JsonRejection>,
) -> Result<Json<AvatarUploadResponse>, AppError> {
    let Json(input) = payload.map_err(AppError::invalid_json)?;
    crate::avatar_models::validate(&input)?;
    let object_key = crate::avatar_models::object_key(auth.principal_id);
    let upload = state
        .avatar_storage
        .presign_upload(
            &object_key,
            Some(&input.content_type),
            input.size_bytes,
            Duration::from_secs(600),
        )
        .await
        .map_err(|error| {
            tracing::error!(%error, "Account avatar upload presigning failed");
            AppError::internal(
                "avatar_storage_error",
                "The Account avatar upload could not be prepared.",
            )
        })?;
    crate::avatar_db::save_key(&state.db, auth.principal_id, &object_key).await?;
    Ok(Json(AvatarUploadResponse {
        upload_url: upload.url,
        object_key,
    }))
}

#[utoipa::path(
    get,
    path = "/api/v1/profile/avatar",
    tag = "profile",
    operation_id = "downloadAccountAvatar",
    security(("identityOAuth2" = ["account:profile:read"])),
    responses(
        (status = 307, description = "Temporary redirect to an Account avatar object"),
        (status = 401, body = crate::error::ErrorEnvelope),
        (status = 403, body = crate::error::ErrorEnvelope),
        (status = 404, body = crate::error::ErrorEnvelope),
    )
)]
pub async fn download_avatar(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedPrincipal>,
) -> Result<Response, AppError> {
    let object_key = crate::avatar_db::fetch_key(&state.db, auth.principal_id)
        .await?
        .ok_or_else(|| {
            AppError::not_found("avatar_not_found", "This Account has no profile avatar.")
        })?;
    let download = state
        .avatar_storage
        .presign_download(&object_key, Duration::from_secs(300))
        .await
        .map_err(|error| {
            tracing::error!(%error, "Account avatar download presigning failed");
            AppError::internal(
                "avatar_storage_error",
                "The Account avatar download could not be prepared.",
            )
        })?;
    Response::builder()
        .status(StatusCode::TEMPORARY_REDIRECT)
        .header(header::LOCATION, download.url)
        .body(axum::body::Body::empty())
        .map_err(|_| AppError::internal("response_error", "The avatar response is invalid."))
}

#[utoipa::path(
    delete,
    path = "/api/v1/profile/avatar",
    tag = "profile",
    operation_id = "deleteAccountAvatar",
    security(("identityOAuth2" = ["account:profile:write"])),
    responses(
        (status = 200, body = SuccessResponse),
        (status = 401, body = crate::error::ErrorEnvelope),
        (status = 403, body = crate::error::ErrorEnvelope),
    )
)]
pub async fn delete_avatar(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedPrincipal>,
) -> Result<Json<SuccessResponse>, AppError> {
    if let Some(object_key) = crate::avatar_db::fetch_key(&state.db, auth.principal_id).await? {
        state
            .avatar_storage
            .delete_objects(&[object_key])
            .await
            .map_err(|error| {
                tracing::error!(%error, "Account avatar deletion failed");
                AppError::internal(
                    "avatar_storage_error",
                    "The Account avatar could not be deleted.",
                )
            })?;
    }
    crate::avatar_db::clear_key(&state.db, auth.principal_id).await?;
    Ok(Json(SuccessResponse::ok()))
}
