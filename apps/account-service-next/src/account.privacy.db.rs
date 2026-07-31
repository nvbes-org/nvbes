use chrono::{Duration, Utc};
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;

pub async fn build_export(db: &PgPool, principal_id: Uuid) -> Result<Value, AppError> {
    sqlx::query_scalar::<_, Value>(
        r#"
        SELECT jsonb_build_object(
          'schema_version', 1,
          'product', 'account',
          'principal_id', $1,
          'exported_at', NOW(),
          'profile', (
            SELECT jsonb_build_object(
              'principal_id', principal_id,
              'firstname', firstname,
              'lastname', lastname,
              'username', username,
              'birthdate', birthdate,
              'region', region,
              'avatar_object_key', avatar_object_key,
              'created_at', created_at,
              'updated_at', updated_at
            )
            FROM account_profiles
            WHERE principal_id = $1
          ),
          'preferences', (
            SELECT to_jsonb(account_preferences) - 'principal_id'
            FROM account_preferences
            WHERE principal_id = $1
          ),
          'notifications', (
            SELECT to_jsonb(account_notifications) - 'principal_id'
            FROM account_notifications
            WHERE principal_id = $1
          ),
          'consents', COALESCE((
            SELECT jsonb_agg(to_jsonb(account_consents) ORDER BY granted_at DESC, id DESC)
            FROM account_consents
            WHERE principal_id = $1
          ), '[]'::jsonb),
          'closure_requests', COALESCE((
            SELECT jsonb_agg(
              jsonb_build_object(
                'id', id,
                'status', status,
                'requested_at', requested_at,
                'updated_at', updated_at,
                'completed_at', completed_at
              )
              ORDER BY requested_at DESC, id DESC
            )
            FROM account_closure_sagas
            WHERE principal_id = $1
          ), '[]'::jsonb)
        )
        "#,
    )
    .bind(principal_id)
    .fetch_one(db)
    .await
    .map_err(AppError::from)
}

pub async fn store_export(
    db: &PgPool,
    principal_id: Uuid,
    document: Value,
) -> Result<(), AppError> {
    let id = Uuid::new_v4();
    let expires_at = Utc::now() + Duration::hours(24);
    let mut transaction = db.begin().await?;
    sqlx::query(
        "DELETE FROM account_privacy_exports WHERE principal_id = $1 AND expires_at <= NOW()",
    )
    .bind(principal_id)
    .execute(&mut *transaction)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO account_privacy_exports
          (id, principal_id, document, expires_at)
        VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(id)
    .bind(principal_id)
    .bind(document)
    .bind(expires_at)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(())
}

pub async fn latest_export(db: &PgPool, principal_id: Uuid) -> Result<Option<Value>, AppError> {
    sqlx::query_scalar::<_, Value>(
        r#"
        SELECT document
        FROM account_privacy_exports
        WHERE principal_id = $1 AND expires_at > NOW()
        ORDER BY completed_at DESC, id DESC
        LIMIT 1
        "#,
    )
    .bind(principal_id)
    .fetch_optional(db)
    .await
    .map_err(AppError::from)
}
