use axum::{Json, Router, extract::State, http::HeaderMap, routing::post};
use chrono::Utc;
use nvbes_product_account::export_event::{
    AccountExportContractError, AccountExportFragmentV1, AccountExportRequestedV1,
};
use serde_json::Value;

use crate::{app::AppState, http::error::AppError};

pub fn router() -> Router<AppState> {
    Router::new().route("/internal/v1/account-exports", post(export_account))
}

async fn export_account(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(command): Json<AccountExportRequestedV1>,
) -> Result<Json<AccountExportFragmentV1>, AppError> {
    if !nvbes_core::http::internal_service::bearer_matches(&headers, &state.internal_service_token)
    {
        return Err(AppError::unauthorized(
            "invalid_internal_token",
            "A valid Cloud internal token is required.",
        ));
    }
    validate_command(&command)?;
    let data = build_fragment(&state.db, command.principal_id).await?;
    Ok(Json(AccountExportFragmentV1::new(
        "cloud",
        command.principal_id,
        data,
    )))
}

fn validate_command(command: &AccountExportRequestedV1) -> Result<(), AppError> {
    match command.validate(Utc::now()) {
        Ok(()) => Ok(()),
        Err(AccountExportContractError::UnsupportedVersion) => Err(AppError::bad_request(
            "unsupported_account_export_event",
            "The Account export event version is not supported.",
        )),
        Err(_) => Err(AppError::bad_request(
            "invalid_account_export_event",
            "The Account export event is invalid.",
        )),
    }
}

async fn build_fragment(db: &sqlx::PgPool, principal_id: uuid::Uuid) -> Result<Value, AppError> {
    sqlx::query_scalar(
        r#"WITH subject AS (
          SELECT id FROM users WHERE identity_subject = $1::text OR id = $1 LIMIT 1
        )
        SELECT jsonb_build_object(
          'user_projection', (SELECT jsonb_build_object(
            'id', id, 'identity_subject', identity_subject, 'status', status,
            'created_at', created_at, 'updated_at', updated_at
          ) FROM users WHERE id = (SELECT id FROM subject)),
          'workspace_memberships', COALESCE((SELECT jsonb_agg(
            jsonb_build_object('workspace_id', wm.workspace_id, 'workspace_name', w.name,
              'role', wm.role, 'status', wm.status, 'created_at', wm.created_at,
              'updated_at', wm.updated_at) ORDER BY wm.created_at DESC)
            FROM workspace_memberships wm JOIN workspaces w ON w.id = wm.workspace_id
            WHERE wm.user_id = (SELECT id FROM subject)), '[]'::jsonb),
          'owned_workspaces', COALESCE((SELECT jsonb_agg(jsonb_build_object(
            'id', id, 'name', name, 'workspace_type', workspace_type,
            'created_at', created_at, 'updated_at', updated_at) ORDER BY created_at DESC)
            FROM workspaces WHERE owner_principal_id = $1), '[]'::jsonb),
          'created_storage_objects', COALESCE((SELECT jsonb_agg(
            to_jsonb(o) - 'object_key' ORDER BY created_at DESC)
            FROM storage_objects o WHERE created_by_principal_id = $1), '[]'::jsonb),
          'created_upload_sessions', COALESCE((SELECT jsonb_agg(
            to_jsonb(u) - 'storage_multipart_upload_id' ORDER BY created_at DESC)
            FROM upload_sessions u WHERE created_by_principal_id = $1), '[]'::jsonb),
          'created_share_links', COALESCE((SELECT jsonb_agg(
            to_jsonb(s) - 'token_hash' ORDER BY created_at DESC)
            FROM share_links s WHERE created_by_principal_id = $1), '[]'::jsonb),
          'created_api_keys', COALESCE((SELECT jsonb_agg(
            to_jsonb(k) - 'key_hash' - 'http_signature_public_key' ORDER BY created_at DESC)
            FROM api_keys k WHERE created_by_principal_id = $1), '[]'::jsonb),
          'api_request_logs', COALESCE((SELECT jsonb_agg(to_jsonb(l) ORDER BY created_at DESC)
            FROM api_request_logs l WHERE actor_principal_id = $1), '[]'::jsonb),
          'audit_events', COALESCE((SELECT jsonb_agg(to_jsonb(a) ORDER BY created_at DESC)
            FROM audit_events a WHERE actor_principal_id = $1), '[]'::jsonb)
        )"#,
    )
    .bind(principal_id)
    .fetch_one(db)
    .await
    .map_err(AppError::from)
}

#[cfg(test)]
mod tests {
    use sqlx::postgres::PgPoolOptions;

    #[tokio::test]
    #[ignore = "requires NVBES_CLOUD_TEST_DATABASE_URL pointing to disposable PostgreSQL"]
    async fn fragment_query_excludes_cloud_secrets() {
        let url = std::env::var("NVBES_CLOUD_TEST_DATABASE_URL").unwrap();
        let db = PgPoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await
            .unwrap();
        let fragment = super::build_fragment(&db, uuid::Uuid::new_v4())
            .await
            .unwrap();
        let encoded = fragment.to_string();
        for secret in ["password_hash", "key_hash", "token_hash", "object_key"] {
            assert!(!encoded.contains(secret));
        }
    }
}
