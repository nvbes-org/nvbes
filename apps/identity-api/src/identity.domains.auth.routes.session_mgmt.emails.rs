use axum::{
    Json,
    extract::{Extension, Path, State},
};
use uuid::Uuid;

use crate::app::AppState;
use crate::domains::auth::email_addresses;
use crate::domains::auth::types::{
    AddSecondaryEmailInput, AddSecondaryEmailResult, DeleteSecondaryEmailResult,
    EmailAddressesResult, PromoteSecondaryEmailResult, ResendSecondaryEmailVerificationResult,
};
use crate::http::error::AppError;
use crate::http::middleware::jwt::AuthContext;

#[utoipa::path(
    get,
    path = "/auth/me/emails",
    tag = "auth",
    responses(
        (status = 200, description = "User email addresses", body = EmailAddressesResult),
        (status = 401, description = "Unauthorized", body = nvbes_core::http::error::ErrorEnvelope),
    ),
)]
pub(crate) async fn me_emails_get(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<EmailAddressesResult>, AppError> {
    Ok(Json(email_addresses::list(&state.db, auth.user_id).await?))
}

#[utoipa::path(
    post,
    path = "/auth/me/emails",
    tag = "auth",
    request_body = AddSecondaryEmailInput,
    responses(
        (status = 200, description = "Secondary email added", body = AddSecondaryEmailResult),
        (status = 400, description = "Validation error", body = nvbes_core::http::error::ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = nvbes_core::http::error::ErrorEnvelope),
        (status = 409, description = "Email already exists", body = nvbes_core::http::error::ErrorEnvelope),
    ),
)]
pub(crate) async fn me_emails_post(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(request): Json<AddSecondaryEmailInput>,
) -> Result<Json<AddSecondaryEmailResult>, AppError> {
    crate::domains::auth::check_rate_limit(
        &state.redis,
        "auth_me_emails_add",
        &format!("user:{}", auth.user_id),
        6,
        std::time::Duration::from_secs(300),
    )
    .await?;
    Ok(Json(
        email_addresses::add_secondary(
            &state.db,
            &state.redis,
            &state.config,
            auth.user_id,
            request,
        )
        .await?,
    ))
}

#[utoipa::path(
    post,
    path = "/auth/me/emails/{emailId}/promote",
    tag = "auth",
    responses(
        (status = 200, description = "Secondary email promoted", body = PromoteSecondaryEmailResult),
        (status = 401, description = "Unauthorized", body = nvbes_core::http::error::ErrorEnvelope),
        (status = 403, description = "Policy violation", body = nvbes_core::http::error::ErrorEnvelope),
        (status = 404, description = "Email not found", body = nvbes_core::http::error::ErrorEnvelope),
    ),
)]
pub(crate) async fn me_email_promote(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(email_id): Path<Uuid>,
) -> Result<Json<PromoteSecondaryEmailResult>, AppError> {
    Ok(Json(
        email_addresses::promote_secondary(&state.db, auth.user_id, email_id).await?,
    ))
}

#[utoipa::path(
    post,
    path = "/auth/me/emails/{emailId}/resend-verification",
    tag = "auth",
    responses(
        (status = 200, description = "Secondary email verification resent", body = ResendSecondaryEmailVerificationResult),
        (status = 400, description = "Email cannot be verified through this flow", body = nvbes_core::http::error::ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = nvbes_core::http::error::ErrorEnvelope),
        (status = 404, description = "Email not found", body = nvbes_core::http::error::ErrorEnvelope),
    ),
)]
pub(crate) async fn me_email_resend_verification(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(email_id): Path<Uuid>,
) -> Result<Json<ResendSecondaryEmailVerificationResult>, AppError> {
    crate::domains::auth::check_rate_limit(
        &state.redis,
        "auth_me_email_resend_verification",
        &format!("user:{}:email:{}", auth.user_id, email_id),
        3,
        std::time::Duration::from_secs(300),
    )
    .await?;
    Ok(Json(
        email_addresses::resend_secondary_verification(
            &state.db,
            &state.redis,
            &state.config,
            auth.user_id,
            email_id,
        )
        .await?,
    ))
}

#[utoipa::path(
    delete,
    path = "/auth/me/emails/{emailId}",
    tag = "auth",
    responses(
        (status = 200, description = "Secondary email deleted", body = DeleteSecondaryEmailResult),
        (status = 401, description = "Unauthorized", body = nvbes_core::http::error::ErrorEnvelope),
        (status = 403, description = "Primary email cannot be deleted", body = nvbes_core::http::error::ErrorEnvelope),
        (status = 404, description = "Email not found", body = nvbes_core::http::error::ErrorEnvelope),
    ),
)]
pub(crate) async fn me_email_delete(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(email_id): Path<Uuid>,
) -> Result<Json<DeleteSecondaryEmailResult>, AppError> {
    Ok(Json(
        email_addresses::delete_secondary(&state.db, auth.user_id, email_id).await?,
    ))
}
