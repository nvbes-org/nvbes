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
            service, service_accounts_db,
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
    let secret_versions =
        service_accounts_db::list_secret_version_summaries(&state.db, tenant_id, &client_id)
            .await?;
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
    let mut tx = state.db.begin().await?;

    let previous_version_id = service_accounts_db::ensure_previous_overlap_version(
        &mut tx,
        tenant_id,
        &client_id,
        &current_hash,
        overlap_ends_at,
    )
    .await?;
    let active_version_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO developer_client_secret_versions (
          tenant_id,
          client_id,
          status,
          client_secret_hash,
          secret_last4,
          created_at
        )
        VALUES ($1, $2, 'active', $3, $4, $5)
        RETURNING id
        "#,
    )
    .bind(tenant_id)
    .bind(&client_id)
    .bind(&client_secret_hash)
    .bind(secret_last4)
    .bind(rotated_at)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        UPDATE developer_secret_rotations
        SET revoked_at = $3
        WHERE tenant_id = $1
          AND client_id = $2
          AND revoked_at IS NULL
        "#,
    )
    .bind(tenant_id)
    .bind(&client_id)
    .bind(rotated_at)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO developer_secret_rotations (
          tenant_id,
          client_id,
          previous_version_id,
          active_version_id,
          previous_version_expires_at,
          overlap_ends_at,
          rotated_by,
          created_at
        )
        VALUES ($1, $2, $3, $4, $5, $5, $6, $7)
        "#,
    )
    .bind(tenant_id)
    .bind(&client_id)
    .bind(previous_version_id)
    .bind(active_version_id)
    .bind(overlap_ends_at)
    .bind(auth.user_id)
    .bind(rotated_at)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "UPDATE oauth_clients SET client_secret_hash = $3, updated_at = $4 WHERE tenant_id = $1 AND client_id = $2",
    )
    .bind(tenant_id)
    .bind(&client_id)
    .bind(&client_secret_hash)
    .bind(rotated_at)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(RotateDeveloperSecretResponse {
        client_id,
        client_secret,
        active_version_id,
        previous_version_id,
        overlap_ends_at,
        rotated_at,
    }))
}

pub async fn revoke_secret_version(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path((client_id, version_id)): Path<(String, Uuid)>,
) -> Result<Json<DeveloperSecretVersionsResponse>, AppError> {
    let tenant_id = service::require_tenant_id(&auth)?;
    let revoked_at = Utc::now();

    sqlx::query(
        r#"
        UPDATE developer_client_secret_versions
        SET status = 'revoked',
            revoked_at = $4
        WHERE tenant_id = $1
          AND client_id = $2
          AND id = $3
        "#,
    )
    .bind(tenant_id)
    .bind(&client_id)
    .bind(version_id)
    .bind(revoked_at)
    .execute(&state.db)
    .await?;

    let secret_versions =
        service_accounts_db::list_secret_version_summaries(&state.db, tenant_id, &client_id)
            .await?;
    Ok(Json(DeveloperSecretVersionsResponse { secret_versions }))
}
