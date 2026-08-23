use sqlx::PgPool;
use uuid::Uuid;

use crate::domains::auth::types::EmailAddressView;
use crate::http::error::AppError;

use super::map;

pub async fn primary_email_policy_hours(db: &PgPool, principal_id: Uuid) -> Result<i32, AppError> {
    let hours = sqlx::query_scalar::<_, i32>(
        r#"
        SELECT COALESCE(t.secondary_email_primary_min_age_hours, 24)
        FROM principals p
        JOIN tenants t ON t.id = p.tenant_id
        WHERE p.id = $1
        "#,
    )
    .bind(principal_id)
    .fetch_one(db)
    .await?;
    Ok(hours)
}

pub async fn active_email_recipients(
    db: &PgPool,
    principal_id: Uuid,
) -> Result<Vec<String>, AppError> {
    let emails = sqlx::query_scalar::<_, String>(
        r#"
        SELECT email
        FROM user_email_addresses
        WHERE principal_id = $1
          AND deleted_at IS NULL
        ORDER BY is_primary DESC, created_at ASC
        "#,
    )
    .bind(principal_id)
    .fetch_all(db)
    .await?;
    Ok(emails)
}

pub async fn verified_security_notification_recipients(
    db: &PgPool,
    principal_id: Uuid,
) -> Result<Vec<String>, AppError> {
    let emails = sqlx::query_scalar::<_, String>(
        r#"
        SELECT email
        FROM user_email_addresses
        WHERE principal_id = $1
          AND deleted_at IS NULL
          AND verified_at IS NOT NULL
        ORDER BY is_primary DESC, created_at ASC
        "#,
    )
    .bind(principal_id)
    .fetch_all(db)
    .await?;
    Ok(emails)
}

pub async fn verified_mfa_eligible_emails(
    db: &PgPool,
    principal_id: Uuid,
) -> Result<Vec<EmailAddressView>, AppError> {
    let min_age_hours = primary_email_policy_hours(db, principal_id).await?;
    let rows = sqlx::query(
        r#"
        SELECT id, principal_id, email, is_primary, verified_at, created_at, updated_at
        FROM user_email_addresses
        WHERE principal_id = $1
          AND deleted_at IS NULL
          AND verified_at IS NOT NULL
          AND (is_primary = TRUE OR created_at <= NOW() - make_interval(hours => $2::int))
        ORDER BY is_primary DESC, created_at ASC
        "#,
    )
    .bind(principal_id)
    .bind(min_age_hours)
    .fetch_all(db)
    .await?;
    Ok(rows.into_iter().map(map::email_address_view).collect())
}
