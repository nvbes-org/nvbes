use axum::{
    Extension, Json,
    extract::{Path, State},
};
use chrono::{Duration, Utc};
use uuid::Uuid;

use crate::{
    access,
    app::DeveloperAppState,
    grpc::{
        pb::nvbes::developer::v1::{
            ListSecretVersionsRequest, RecordSecretRotationRequest, RevokeSecretVersionRequest,
            SecretVersion,
        },
        secrets,
    },
    http::{
        auth::{DeveloperAuth, require_recent_step_up},
        context::{optional_time, request_context, time, uuid},
        error::AppError,
        types::{
            DeveloperSecretVersionSummary, DeveloperSecretVersionsResponse,
            DeveloperServiceAccountSummary, DeveloperServiceAccountsResponse,
            RotateDeveloperSecretInput, RotateDeveloperSecretResponse,
        },
    },
    rbac::DeveloperPermission,
};

#[utoipa::path(get, path = "/developer/console/service-accounts", tag = "developer-console", responses((status = 200, description = "Service accounts")))]
pub async fn list_service_accounts(
    State(state): State<DeveloperAppState>,
    Extension(auth): Extension<DeveloperAuth>,
) -> Result<Json<DeveloperServiceAccountsResponse>, AppError> {
    access::require_permission(
        &state.db,
        &auth,
        DeveloperPermission::ConsoleServiceAccountsRead,
    )
    .await?;
    let service_accounts = sqlx::query_as::<_, DeveloperServiceAccountSummary>(
        r#"
        SELECT
          sa.principal_id,
          sa.name,
          sa.description,
          COALESCE((
            SELECT wm.role::text
            FROM workspace_memberships wm
            WHERE wm.workspace_id = sa.workspace_id
              AND wm.principal_id = sa.principal_id
              AND wm.status = 'active'
            LIMIT 1
          ), 'viewer') AS role,
          CASE WHEN p.revoked_at IS NULL THEN 'active' ELSE 'revoked' END AS status,
          sa.workspace_id,
          COUNT(oc.id)::bigint AS oauth_client_count,
          sa.last_rotated_at
        FROM service_accounts sa
        INNER JOIN principals p ON p.id = sa.principal_id
        LEFT JOIN oauth_clients oc ON oc.client_id = sa.client_id
        WHERE sa.tenant_id = $1
        GROUP BY sa.principal_id, sa.name, sa.description, p.revoked_at,
                 sa.workspace_id, sa.last_rotated_at
        ORDER BY sa.name ASC
        "#,
    )
    .bind(auth.tenant_id)
    .fetch_all(&state.db)
    .await?;
    Ok(Json(DeveloperServiceAccountsResponse { service_accounts }))
}

#[utoipa::path(get, path = "/developer/console/oauth-clients/{clientId}/secrets", tag = "developer-console", responses((status = 200, description = "Secret versions")))]
pub async fn list_secret_versions(
    State(state): State<DeveloperAppState>,
    Extension(auth): Extension<DeveloperAuth>,
    Path(client_id): Path<String>,
) -> Result<Json<DeveloperSecretVersionsResponse>, AppError> {
    access::require_permission(
        &state.db,
        &auth,
        DeveloperPermission::ConsoleServiceAccountsRead,
    )
    .await?;
    ensure_client(&state.db, auth.tenant_id, &client_id).await?;
    let response = secrets::list_secret_versions(
        &state.db,
        ListSecretVersionsRequest {
            context: Some(request_context(&auth)),
            tenant_id: auth.tenant_id.to_string(),
            client_id,
        },
    )
    .await?;
    Ok(Json(secret_versions(response.secret_versions)?))
}

#[utoipa::path(post, path = "/developer/console/oauth-clients/{clientId}/secrets/rotation", tag = "developer-console", responses((status = 200, description = "Secret rotated")))]
pub async fn rotate_secret(
    State(state): State<DeveloperAppState>,
    Extension(auth): Extension<DeveloperAuth>,
    Path(client_id): Path<String>,
    Json(input): Json<RotateDeveloperSecretInput>,
) -> Result<Json<RotateDeveloperSecretResponse>, AppError> {
    require_recent_step_up(&auth)?;
    access::require_permission(&state.db, &auth, DeveloperPermission::SecretsRotate).await?;
    if input.overlap_hours < 1 {
        return Err(AppError::bad_request(
            "invalid_overlap_window",
            "Overlap must be at least 1 hour.",
        ));
    }

    let current_hash = current_secret_hash(&state.db, auth.tenant_id, &client_id).await?;
    let rotated_at = Utc::now();
    let overlap_ends_at = rotated_at + Duration::hours(input.overlap_hours);
    let client_secret = format!(
        "gxo_{}_{}",
        Uuid::new_v4().simple(),
        Uuid::new_v4().simple()
    );
    let new_hash = nvbes_product_identity::oauth::hash_client_secret(&client_secret)?;
    let last4 = client_secret
        .get(client_secret.len().saturating_sub(4)..)
        .unwrap_or(&client_secret)
        .to_string();
    let rotation = secrets::record_secret_rotation(
        &state.db,
        auth.user_id,
        RecordSecretRotationRequest {
            context: Some(request_context(&auth)),
            tenant_id: auth.tenant_id.to_string(),
            client_id: client_id.clone(),
            current_secret_hash: current_hash,
            new_secret_hash: new_hash.clone(),
            new_secret_last4: last4,
            overlap_ends_at: overlap_ends_at.to_rfc3339(),
            rotated_at: rotated_at.to_rfc3339(),
        },
    )
    .await?;

    sqlx::query(
        "UPDATE oauth_clients SET client_secret_hash = $3, updated_at = $4 \
         WHERE tenant_id = $1 AND client_id = $2",
    )
    .bind(auth.tenant_id)
    .bind(&client_id)
    .bind(new_hash)
    .bind(rotated_at)
    .execute(&state.db)
    .await?;

    Ok(Json(RotateDeveloperSecretResponse {
        client_id: rotation.client_id,
        client_secret,
        active_version_id: uuid(&rotation.active_version_id, "active version id")?,
        previous_version_id: uuid(&rotation.previous_version_id, "previous version id")?,
        overlap_ends_at: time(&rotation.overlap_ends_at, "overlap_ends_at")?,
        rotated_at: time(&rotation.rotated_at, "rotated_at")?,
    }))
}

#[utoipa::path(post, path = "/developer/console/oauth-clients/{clientId}/secrets/{versionId}/revoke", tag = "developer-console", responses((status = 200, description = "Secret version revoked")))]
pub async fn revoke_secret_version(
    State(state): State<DeveloperAppState>,
    Extension(auth): Extension<DeveloperAuth>,
    Path((client_id, version_id)): Path<(String, Uuid)>,
) -> Result<Json<DeveloperSecretVersionsResponse>, AppError> {
    require_recent_step_up(&auth)?;
    access::require_permission(
        &state.db,
        &auth,
        DeveloperPermission::ConsoleServiceAccountsManage,
    )
    .await?;
    ensure_client(&state.db, auth.tenant_id, &client_id).await?;
    let revoked = secrets::revoke_secret_version(
        &state.db,
        RevokeSecretVersionRequest {
            context: Some(request_context(&auth)),
            tenant_id: auth.tenant_id.to_string(),
            version_id: version_id.to_string(),
        },
    )
    .await?;
    if revoked.client_id != client_id {
        return Err(AppError::not_found(
            "developer_secret_not_found",
            "Developer secret not found for this client.",
        ));
    }
    let response = secrets::list_secret_versions(
        &state.db,
        ListSecretVersionsRequest {
            context: Some(request_context(&auth)),
            tenant_id: auth.tenant_id.to_string(),
            client_id,
        },
    )
    .await?;
    Ok(Json(secret_versions(response.secret_versions)?))
}

async fn ensure_client(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
    client_id: &str,
) -> Result<(), AppError> {
    current_secret_hash(db, tenant_id, client_id)
        .await
        .map(|_| ())
}

async fn current_secret_hash(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
    client_id: &str,
) -> Result<String, AppError> {
    sqlx::query_scalar(
        "SELECT client_secret_hash FROM oauth_clients WHERE tenant_id = $1 AND client_id = $2",
    )
    .bind(tenant_id)
    .bind(client_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::not_found("oauth_client_not_found", "OAuth client not found."))
}

fn secret_versions(
    versions: Vec<SecretVersion>,
) -> Result<DeveloperSecretVersionsResponse, AppError> {
    let secret_versions = versions
        .into_iter()
        .map(|version| {
            Ok(DeveloperSecretVersionSummary {
                id: uuid(&version.id, "secret version id")?,
                client_id: version.client_id,
                status: version.status,
                secret_last4: version.secret_last4,
                created_at: time(&version.created_at, "created_at")?,
                expires_at: optional_time(&version.expires_at, "expires_at")?,
                revoked_at: optional_time(&version.revoked_at, "revoked_at")?,
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    Ok(DeveloperSecretVersionsResponse { secret_versions })
}
