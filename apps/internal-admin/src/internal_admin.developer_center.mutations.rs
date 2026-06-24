use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

use crate::billing_admin_types::BackofficeAccess;
use crate::developer_center_types::{DeveloperActionResult, action_result};
use crate::developer_center_validation::{validate_client_id, validate_reason};
use crate::error::AppError;

pub(crate) async fn revoke_client(
    db: &PgPool,
    access: BackofficeAccess,
    workspace_id: Uuid,
    client_id: String,
    reason: String,
) -> Result<DeveloperActionResult, AppError> {
    validate_client_id(&client_id)?;
    validate_reason(&reason)?;
    let mut tx = db.begin().await?;
    let oauth_client_id = sqlx::query_scalar::<_, Uuid>(
        "UPDATE oauth_clients
         SET revoked_at = NOW(), updated_at = NOW()
         WHERE tenant_id = $1 AND client_id = $2 AND revoked_at IS NULL
         RETURNING id",
    )
    .bind(access.tenant_id)
    .bind(&client_id)
    .fetch_optional(tx.as_mut())
    .await?
    .ok_or_else(|| {
        AppError::conflict(
            "developer_client_not_revokable",
            "Developer client is missing or already revoked.",
        )
    })?;
    let action_id = insert_developer_action(
        &mut tx,
        access,
        workspace_id,
        DeveloperActionInput {
            action_kind: "revoke_client",
            client_id: Some(client_id),
            marketplace_app_id: None,
            target_id: Some(oauth_client_id),
            status: "applied",
            reason,
            metadata: json!({ "oauth_client_id": oauth_client_id }),
        },
    )
    .await?;
    insert_developer_audit(
        &mut tx,
        access,
        "developer.client.revoked",
        "oauth_client",
        oauth_client_id,
        action_id,
        json!({
            "object_links": {
                "oauth_client_id": oauth_client_id,
                "workspace_id": workspace_id,
            },
            "changes": [
                {
                    "field": "revoked_at",
                    "before": null,
                    "after": "recorded",
                }
            ],
        }),
    )
    .await?;
    tx.commit().await?;
    Ok(action_result(
        action_id,
        "revoke_client",
        "applied",
        "developer.client.revoked",
    ))
}

pub(crate) async fn rotate_secret(
    db: &PgPool,
    access: BackofficeAccess,
    workspace_id: Uuid,
    client_id: String,
    reason: String,
) -> Result<DeveloperActionResult, AppError> {
    validate_client_id(&client_id)?;
    validate_reason(&reason)?;
    let mut tx = db.begin().await?;
    let previous_version_id = sqlx::query_scalar::<_, Uuid>(
        "UPDATE developer_client_secret_versions
         SET status = 'overlap', expires_at = NOW() + INTERVAL '7 days'
         WHERE tenant_id = $1 AND client_id = $2 AND status = 'active' AND revoked_at IS NULL
         RETURNING id",
    )
    .bind(access.tenant_id)
    .bind(&client_id)
    .fetch_optional(tx.as_mut())
    .await?
    .ok_or_else(|| {
        AppError::conflict(
            "developer_secret_not_rotatable",
            "Developer client does not have an active secret to rotate.",
        )
    })?;
    let active_version_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO developer_client_secret_versions (
           tenant_id, client_id, status, client_secret_hash, secret_last4, expires_at
         ) VALUES ($1, $2, 'active', $3, $4, NOW() + INTERVAL '365 days')
         RETURNING id",
    )
    .bind(access.tenant_id)
    .bind(&client_id)
    .bind(format!("backoffice-rotated-{}", Uuid::new_v4()))
    .bind(rotation_last4())
    .fetch_one(tx.as_mut())
    .await?;
    let rotation_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO developer_secret_rotations (
           tenant_id, client_id, previous_version_id, active_version_id,
           previous_version_expires_at, overlap_ends_at, rotated_by
         ) VALUES (
           $1, $2, $3, $4, NOW() + INTERVAL '7 days', NOW() + INTERVAL '7 days', $5
         ) RETURNING id",
    )
    .bind(access.tenant_id)
    .bind(&client_id)
    .bind(previous_version_id)
    .bind(active_version_id)
    .bind(access.actor_principal_id)
    .fetch_one(tx.as_mut())
    .await?;
    let action_id = insert_developer_action(
        &mut tx,
        access,
        workspace_id,
        DeveloperActionInput {
            action_kind: "rotate_secret",
            client_id: Some(client_id),
            marketplace_app_id: None,
            target_id: Some(rotation_id),
            status: "applied",
            reason,
            metadata: json!({
                "previous_version_id": previous_version_id,
                "active_version_id": active_version_id
            }),
        },
    )
    .await?;
    insert_developer_audit(
        &mut tx,
        access,
        "developer.secret.rotated",
        "developer_secret_rotation",
        rotation_id,
        action_id,
        json!({
            "object_links": {
                "rotation_id": rotation_id,
                "previous_version_id": previous_version_id,
                "active_version_id": active_version_id,
                "workspace_id": workspace_id,
            },
            "changes": [
                {
                    "field": "previous_secret.status",
                    "before": "active",
                    "after": "overlap",
                },
                {
                    "field": "active_secret",
                    "before": null,
                    "after": "created",
                }
            ],
        }),
    )
    .await?;
    tx.commit().await?;
    Ok(action_result(
        action_id,
        "rotate_secret",
        "applied",
        "developer.secret.rotated",
    ))
}

pub(crate) async fn approve_marketplace_app(
    db: &PgPool,
    access: BackofficeAccess,
    workspace_id: Uuid,
    app_id: Uuid,
    reason: String,
) -> Result<DeveloperActionResult, AppError> {
    validate_reason(&reason)?;
    let mut tx = db.begin().await?;
    let marketplace_app_id = sqlx::query_scalar::<_, Uuid>(
        "UPDATE developer_marketplace_apps
         SET status = 'approved', reviewed_by = $1, review_reason = $2, updated_at = NOW()
         WHERE tenant_id = $3 AND id = $4 AND status = 'pending'
         RETURNING id",
    )
    .bind(access.actor_principal_id)
    .bind(&reason)
    .bind(access.tenant_id)
    .bind(app_id)
    .fetch_optional(tx.as_mut())
    .await?
    .ok_or_else(|| {
        AppError::conflict(
            "marketplace_app_not_approvable",
            "Marketplace app is missing or not pending.",
        )
    })?;
    let action_id = insert_developer_action(
        &mut tx,
        access,
        workspace_id,
        DeveloperActionInput {
            action_kind: "approve_marketplace_app",
            client_id: None,
            marketplace_app_id: Some(marketplace_app_id),
            target_id: Some(marketplace_app_id),
            status: "approved",
            reason,
            metadata: json!({}),
        },
    )
    .await?;
    insert_developer_audit(
        &mut tx,
        access,
        "developer.marketplace_app.approved",
        "developer_marketplace_app",
        marketplace_app_id,
        action_id,
        json!({
            "object_links": {
                "marketplace_app_id": marketplace_app_id,
                "workspace_id": workspace_id,
            },
            "changes": [
                {
                    "field": "status",
                    "before": "pending",
                    "after": "approved",
                },
                {
                    "field": "reviewed_by",
                    "before": null,
                    "after": access.actor_principal_id,
                },
                {
                    "field": "review_reason",
                    "before": null,
                    "after": "recorded",
                }
            ],
        }),
    )
    .await?;
    tx.commit().await?;
    Ok(action_result(
        action_id,
        "approve_marketplace_app",
        "approved",
        "developer.marketplace_app.approved",
    ))
}

struct DeveloperActionInput {
    action_kind: &'static str,
    client_id: Option<String>,
    marketplace_app_id: Option<Uuid>,
    target_id: Option<Uuid>,
    status: &'static str,
    reason: String,
    metadata: Value,
}

async fn insert_developer_action(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    access: BackofficeAccess,
    workspace_id: Uuid,
    input: DeveloperActionInput,
) -> Result<Uuid, AppError> {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO internal_admin_developer_actions (
           tenant_id, workspace_id, actor_principal_id, action_kind, client_id,
           marketplace_app_id, target_id, status, reason, metadata
         ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
         RETURNING id",
    )
    .bind(access.tenant_id)
    .bind(workspace_id)
    .bind(access.actor_principal_id)
    .bind(input.action_kind)
    .bind(input.client_id)
    .bind(input.marketplace_app_id)
    .bind(input.target_id)
    .bind(input.status)
    .bind(input.reason)
    .bind(input.metadata)
    .fetch_one(tx.as_mut())
    .await
    .map_err(AppError::from)
}

async fn insert_developer_audit(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    access: BackofficeAccess,
    action: &'static str,
    target_type: &'static str,
    target_id: Uuid,
    action_id: Uuid,
    metadata: Value,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO audit_events (
           tenant_id, actor_principal_id, action, target_type, target_id, metadata, event_hash
         ) VALUES (
           $1, $2, $3, $4, $5,
           jsonb_build_object('developer_action_id', $6) || $7::jsonb,
           gen_random_uuid()::text
         )",
    )
    .bind(access.tenant_id)
    .bind(access.actor_principal_id)
    .bind(action)
    .bind(target_type)
    .bind(target_id)
    .bind(action_id)
    .bind(metadata)
    .execute(tx.as_mut())
    .await?;
    Ok(())
}

fn rotation_last4() -> String {
    Uuid::new_v4().to_string()[..4].to_string()
}
