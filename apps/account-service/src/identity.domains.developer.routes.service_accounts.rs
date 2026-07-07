use axum::{
    Extension, Json,
    extract::{Path, State},
};
use chrono::{Duration, Utc};
use uuid::Uuid;

use crate::{
    app::AppState,
    domains::{
        developer::{
            grpc, service, service_accounts_db,
            types::{
                DeveloperSecretVersionsResponse, DeveloperServiceAccountsResponse,
                RotateDeveloperSecretInput, RotateDeveloperSecretResponse,
            },
        },
        oauth,
    },
    http::{error::AppError, middleware::jwt::AuthContext},
};

pub async fn list_service_accounts(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<DeveloperServiceAccountsResponse>, AppError> {
    let tenant_id = service::require_tenant_id(&auth)?;
    let service_accounts =
        service_accounts_db::list_service_account_summaries(&state.db, tenant_id).await?;
    Ok(Json(DeveloperServiceAccountsResponse { service_accounts }))
}

pub async fn list_secret_versions(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(client_id): Path<String>,
) -> Result<Json<DeveloperSecretVersionsResponse>, AppError> {
    let tenant_id = service::require_tenant_id(&auth)?;
    service_accounts_db::ensure_oauth_client_in_tenant(&state.db, tenant_id, &client_id).await?;
    let secret_versions = grpc::list_secret_versions(tenant_id, auth.user_id, client_id).await?;
    Ok(Json(DeveloperSecretVersionsResponse { secret_versions }))
}

pub async fn rotate_secret(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(client_id): Path<String>,
    Json(input): Json<RotateDeveloperSecretInput>,
) -> Result<Json<RotateDeveloperSecretResponse>, AppError> {
    if input.overlap_hours < 1 {
        return Err(AppError::bad_request(
            "invalid_overlap_window",
            "Overlap must be at least 1 hour",
        ));
    }

    let tenant_id = service::require_tenant_id(&auth)?;
    let current_hash =
        service_accounts_db::current_client_secret_hash(&state.db, tenant_id, &client_id).await?;
    let rotated_at = Utc::now();
    let overlap_ends_at = rotated_at + Duration::hours(input.overlap_hours);
    let client_secret = format!(
        "gxo_{}_{}",
        Uuid::new_v4().simple(),
        Uuid::new_v4().simple()
    );
    let client_secret_hash = oauth::hash_client_secret(&client_secret)?;
    let secret_last4 = service_accounts_db::secret_last4(&client_secret);
    let rotation = grpc::record_secret_rotation(
        tenant_id,
        auth.user_id,
        client_id.clone(),
        current_hash,
        client_secret_hash.clone(),
        secret_last4.to_string(),
        overlap_ends_at,
        rotated_at,
    )
    .await?;

    sqlx::query(
        "UPDATE oauth_clients SET client_secret_hash = $3, updated_at = $4 WHERE tenant_id = $1 AND client_id = $2",
    )
    .bind(tenant_id)
    .bind(&client_id)
    .bind(&client_secret_hash)
    .bind(rotated_at)
    .execute(&state.db)
    .await?;

    Ok(Json(RotateDeveloperSecretResponse {
        client_id: rotation.client_id,
        client_secret,
        active_version_id: rotation.active_version_id,
        previous_version_id: rotation.previous_version_id,
        overlap_ends_at: rotation.overlap_ends_at,
        rotated_at: rotation.rotated_at,
    }))
}

pub async fn revoke_secret_version(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path((client_id, version_id)): Path<(String, Uuid)>,
) -> Result<Json<DeveloperSecretVersionsResponse>, AppError> {
    let tenant_id = service::require_tenant_id(&auth)?;
    service_accounts_db::ensure_oauth_client_in_tenant(&state.db, tenant_id, &client_id).await?;
    let revoked_client_id =
        grpc::revoke_secret_version(tenant_id, auth.user_id, version_id).await?;
    if revoked_client_id != client_id {
        return Err(AppError::not_found(
            "developer_secret_not_found",
            "Developer secret not found for this client.",
        ));
    }
    let secret_versions = grpc::list_secret_versions(tenant_id, auth.user_id, client_id).await?;
    Ok(Json(DeveloperSecretVersionsResponse { secret_versions }))
}
