use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
};
use uuid::Uuid;

use nvbes_core::http::error::ErrorEnvelope;

use crate::{
    app::AppState,
    domains::authz::{ResourceContext, WorkspaceAction, authorize_workspace_action},
    http::error::AppError,
};

use super::request_meta::PublicApiRequestMeta;
use super::types::{ApiKeyListResponse, RevokeApiKeyResponse};

#[utoipa::path(
    get,
    path = "/workspaces/{workspaceId}/api-keys",
    tag = "api-keys",
    params(
        ("workspaceId" = Uuid, Path, description = "Workspace ID"),
    ),
    responses(
        (status = 200, description = "List API keys", body = ApiKeyListResponse),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub async fn list_api_keys(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
) -> Result<Json<ApiKeyListResponse>, AppError> {
    let access = authorize_workspace_action(
        &state.db,
        &headers,
        workspace_id,
        WorkspaceAction::UpdateWorkspaceSettings,
        ResourceContext::default(),
    )
    .await?;
    Ok(Json(
        crate::domains::public_api::list_api_keys(&state.db, &access).await?,
    ))
}

pub async fn create_api_key() -> Result<(), AppError> {
    Err(AppError::new(
        axum::http::StatusCode::GONE,
        "api_key_creation_disabled",
        "New Cloud Service keys are disabled. Create a workspace service account in Identity and attach an OAuth client instead.",
    ))
}

#[utoipa::path(
    delete,
    path = "/workspaces/{workspaceId}/api-keys/{apiKeyId}",
    tag = "api-keys",
    params(
        ("workspaceId" = Uuid, Path, description = "Workspace ID"),
        ("apiKeyId" = Uuid, Path, description = "API key ID"),
    ),
    responses(
        (status = 200, description = "API key revoked", body = RevokeApiKeyResponse),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 404, description = "Not found", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub async fn revoke_api_key(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((workspace_id, api_key_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<RevokeApiKeyResponse>, AppError> {
    let meta = PublicApiRequestMeta::from_headers(&headers);
    let access = authorize_workspace_action(
        &state.db,
        &headers,
        workspace_id,
        WorkspaceAction::UpdateWorkspaceSettings,
        ResourceContext::default(),
    )
    .await?;
    Ok(Json(
        crate::domains::public_api::revoke_api_key(
            &state.db,
            &access,
            api_key_id,
            meta.ip_owned(),
            meta.user_agent_owned(),
        )
        .await?,
    ))
}

#[cfg(test)]
mod tests {
    use super::create_api_key;

    #[tokio::test]
    async fn create_api_key_returns_gone() {
        let error = create_api_key()
            .await
            .expect_err("legacy api key creation must remain disabled");

        assert_eq!(error.status, axum::http::StatusCode::GONE);
        assert_eq!(error.code, "api_key_creation_disabled");
    }
}
