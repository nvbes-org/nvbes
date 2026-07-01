use crate::app::AppState;
use crate::http::middleware::jwt::AuthContext;
use axum::{
    Json, Router,
    extract::{Extension, Path, State},
};

#[path = "identity.domains.auth.routes.session_mgmt.preferences.rs"]
mod preferences;
#[path = "identity.domains.auth.routes.session_mgmt.router.rs"]
mod router_impl;

#[path = "identity.domains.auth.routes.session_mgmt.accounts.rs"]
pub mod accounts;
#[path = "identity.domains.auth.routes.session_mgmt.emails.rs"]
pub mod emails;
#[path = "identity.domains.auth.routes.session_mgmt.export.rs"]
pub mod export;
#[path = "identity.domains.auth.routes.session_mgmt.profile.rs"]
pub mod profile;
#[path = "identity.domains.auth.routes.session_mgmt.sessions.rs"]
pub mod sessions;
#[path = "identity.domains.auth.routes.session_mgmt.step_up.rs"]
pub mod step_up;
#[path = "identity.domains.auth.routes.session_mgmt.switch_workspace.rs"]
pub mod switch_workspace;

pub fn router(state: &AppState) -> Router<AppState> {
    router_impl::router(state)
}

#[utoipa::path(
    get,
    path = "/auth/accounts",
    tag = "auth",
    responses(
        (status = 200, description = "Active accounts retrieved", body = accounts::AccountChooserResult),
    ),
)]
pub(crate) async fn get_accounts(
    State(state): State<AppState>,
    uri: axum::http::Uri,
    headers: axum::http::HeaderMap,
) -> Result<axum::response::Response, crate::http::error::AppError> {
    accounts::get_accounts(State(state), uri, headers).await
}

pub(crate) async fn forget_account_cookie(
    State(state): State<AppState>,
    Path(authuser): Path<String>,
) -> Result<axum::response::Response, crate::http::error::AppError> {
    accounts::forget_account_cookie(State(state), Path(authuser)).await
}

#[utoipa::path(
    post,
    path = "/auth/logout",
    tag = "auth",
    responses(
        (status = 200, description = "Logout successful", body = crate::domains::auth::types::LogoutResult),
        (status = 401, description = "Unauthorized", body = nvbes_core::http::error::ErrorEnvelope),
    ),
)]
pub(crate) async fn logout(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    headers: axum::http::HeaderMap,
) -> Result<axum::response::Response, crate::http::error::AppError> {
    sessions::logout(State(state), Extension(auth), headers).await
}

#[utoipa::path(
    get,
    path = "/auth/sessions",
    tag = "auth",
    responses(
        (status = 200, description = "List of active sessions", body = crate::domains::auth::types::SessionsResult),
        (status = 401, description = "Unauthorized", body = nvbes_core::http::error::ErrorEnvelope),
    ),
)]
pub(crate) async fn list_sessions(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<crate::domains::auth::types::SessionsResult>, crate::http::error::AppError> {
    sessions::list_sessions(State(state), Extension(auth)).await
}

#[utoipa::path(
    delete,
    path = "/auth/sessions/{sessionId}",
    tag = "auth",
    responses(
        (status = 200, description = "Session revoked", body = crate::domains::auth::types::LogoutResult),
        (status = 401, description = "Unauthorized", body = nvbes_core::http::error::ErrorEnvelope),
        (status = 404, description = "Session not found", body = nvbes_core::http::error::ErrorEnvelope),
    ),
)]
pub(crate) async fn revoke_session(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(session_id): Path<uuid::Uuid>,
) -> Result<Json<crate::domains::auth::types::LogoutResult>, crate::http::error::AppError> {
    sessions::revoke_session(State(state), Extension(auth), Path(session_id)).await
}

#[utoipa::path(
    post,
    path = "/auth/sessions/revoke-others",
    tag = "auth",
    responses(
        (status = 200, description = "Other sessions revoked", body = crate::domains::auth::types::LogoutResult),
        (status = 401, description = "Unauthorized", body = nvbes_core::http::error::ErrorEnvelope),
    ),
)]
pub(crate) async fn revoke_all_other_sessions(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<crate::domains::auth::types::LogoutResult>, crate::http::error::AppError> {
    sessions::revoke_all_other_sessions(State(state), Extension(auth)).await
}

pub(crate) async fn me_emails_get(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<crate::domains::auth::types::EmailAddressesResult>, crate::http::error::AppError> {
    emails::me_emails_get(State(state), Extension(auth)).await
}

pub(crate) async fn me_emails_post(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(request): Json<crate::domains::auth::types::AddSecondaryEmailInput>,
) -> Result<Json<crate::domains::auth::types::AddSecondaryEmailResult>, crate::http::error::AppError>
{
    emails::me_emails_post(State(state), Extension(auth), Json(request)).await
}

pub(crate) async fn me_email_promote(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(email_id): Path<uuid::Uuid>,
) -> Result<
    Json<crate::domains::auth::types::PromoteSecondaryEmailResult>,
    crate::http::error::AppError,
> {
    emails::me_email_promote(State(state), Extension(auth), Path(email_id)).await
}

pub(crate) async fn me_email_resend_verification(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(email_id): Path<uuid::Uuid>,
) -> Result<
    Json<crate::domains::auth::types::ResendSecondaryEmailVerificationResult>,
    crate::http::error::AppError,
> {
    emails::me_email_resend_verification(State(state), Extension(auth), Path(email_id)).await
}

pub(crate) async fn me_email_delete(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(email_id): Path<uuid::Uuid>,
) -> Result<
    Json<crate::domains::auth::types::DeleteSecondaryEmailResult>,
    crate::http::error::AppError,
> {
    emails::me_email_delete(State(state), Extension(auth), Path(email_id)).await
}

#[utoipa::path(
    get,
    path = "/auth/me",
    tag = "auth",
    responses(
        (status = 200, description = "Current user info", body = crate::domains::auth::types::MeResult),
        (status = 401, description = "Unauthorized", body = nvbes_core::http::error::ErrorEnvelope),
    ),
)]
pub(crate) async fn me(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<crate::domains::auth::types::MeResult>, crate::http::error::AppError> {
    profile::me(State(state), Extension(auth)).await
}

#[utoipa::path(
    post,
    path = "/auth/me/export",
    tag = "auth",
    responses(
        (status = 200, description = "Data export request recorded", body = crate::domains::auth::types::DataExportResult),
        (status = 401, description = "Unauthorized", body = nvbes_core::http::error::ErrorEnvelope),
        (status = 429, description = "Rate limited", body = nvbes_core::http::error::ErrorEnvelope),
    ),
)]
pub(crate) async fn me_export(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<crate::domains::auth::types::DataExportResult>, crate::http::error::AppError> {
    export::me_export(State(state), Extension(auth)).await
}

#[utoipa::path(
    get,
    path = "/auth/me/export",
    tag = "auth",
    responses(
        (status = 200, description = "Prepared personal data export", content_type = "application/json"),
        (status = 401, description = "Unauthorized", body = nvbes_core::http::error::ErrorEnvelope),
        (status = 404, description = "No prepared export available", body = nvbes_core::http::error::ErrorEnvelope),
        (status = 429, description = "Rate limited", body = nvbes_core::http::error::ErrorEnvelope),
    ),
)]
pub(crate) async fn me_export_download(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<axum::response::Response, crate::http::error::AppError> {
    export::me_export_download(State(state), Extension(auth)).await
}

#[utoipa::path(
    post,
    path = "/auth/me/delete",
    tag = "auth",
    responses(
        (status = 200, description = "Account deleted successfully", body = crate::domains::auth::types::DeleteAccountResult),
        (status = 401, description = "Unauthorized", body = nvbes_core::http::error::ErrorEnvelope),
        (status = 429, description = "Rate limited", body = nvbes_core::http::error::ErrorEnvelope),
    ),
)]
pub(crate) async fn me_delete(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<crate::domains::auth::types::DeleteAccountResult>, crate::http::error::AppError> {
    profile::me_delete(State(state), Extension(auth)).await
}

pub(crate) async fn me_update(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(request): Json<crate::domains::auth::types::UpdateProfileInput>,
) -> Result<Json<crate::domains::auth::types::UpdateProfileResult>, crate::http::error::AppError> {
    preferences::me_update(State(state), Extension(auth), Json(request)).await
}

pub(crate) async fn me_preferences_get(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<crate::domains::auth::types::UserPreferences>, crate::http::error::AppError> {
    preferences::me_preferences_get(State(state), Extension(auth)).await
}

pub(crate) async fn me_preferences_put(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(request): Json<crate::domains::auth::types::UserPreferences>,
) -> Result<Json<crate::domains::auth::types::UserPreferences>, crate::http::error::AppError> {
    preferences::me_preferences_put(State(state), Extension(auth), Json(request)).await
}

pub(crate) async fn me_notifications_get(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<crate::domains::auth::types::UserNotifications>, crate::http::error::AppError> {
    preferences::me_notifications_get(State(state), Extension(auth)).await
}

pub(crate) async fn me_notifications_put(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(request): Json<crate::domains::auth::types::UserNotifications>,
) -> Result<Json<crate::domains::auth::types::UserNotifications>, crate::http::error::AppError> {
    preferences::me_notifications_put(State(state), Extension(auth), Json(request)).await
}

#[utoipa::path(
    post,
    path = "/auth/step-up",
    tag = "auth",
    request_body = step_up::StepUpRequest,
    responses(
        (status = 200, description = "Step-up successful", body = crate::domains::auth::types::StepUpResult),
        (status = 401, description = "Unauthorized or invalid credentials", body = nvbes_core::http::error::ErrorEnvelope),
    ),
)]
pub(crate) async fn step_up(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(request): Json<step_up::StepUpRequest>,
) -> Result<Json<crate::domains::auth::types::StepUpResult>, crate::http::error::AppError> {
    step_up::step_up(State(state), Extension(auth), Json(request)).await
}

#[utoipa::path(
    post,
    path = "/auth/workspaces/{workspaceId}/switch",
    tag = "auth",
    request_body = switch_workspace::SwitchWorkspaceRequest,
    responses(
        (status = 200, description = "Workspace switched", body = crate::domains::auth::types::SwitchWorkspaceResult),
        (status = 401, description = "Unauthorized or invalid credentials", body = nvbes_core::http::error::ErrorEnvelope),
        (status = 404, description = "Workspace not found", body = nvbes_core::http::error::ErrorEnvelope),
    ),
)]
pub(crate) async fn switch_workspace(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(workspace_id): Path<uuid::Uuid>,
    Json(request): Json<switch_workspace::SwitchWorkspaceRequest>,
) -> Result<Json<crate::domains::auth::types::SwitchWorkspaceResult>, crate::http::error::AppError>
{
    switch_workspace::switch_workspace(
        State(state),
        Extension(auth),
        Path(workspace_id),
        Json(request),
    )
    .await
}
