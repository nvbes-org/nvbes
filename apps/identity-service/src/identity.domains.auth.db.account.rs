use chrono::{DateTime, Utc};
use nvbes_core::config::AppConfig;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domains::auth::{email_verification, password, types::DEFAULT_DISPLAY_NAME};
use crate::http::error::AppError;

pub async fn create_user_account(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    config: &AppConfig,
    email: String,
    data_region: Option<String>,
    password_hash: String,
    verification_token: String,
    registration_enrollment_token_hash: String,
    registration_enrollment_expires_at: DateTime<Utc>,
    ip: Option<String>,
    user_agent: Option<String>,
    legal_documents_accepted: bool,
    marketing_emails_accepted: bool,
) -> Result<(Uuid, DateTime<Utc>), AppError> {
    if !legal_documents_accepted {
        return Err(AppError::bad_request(
            "legal_documents_required",
            "Legal documents must be accepted to create an account.",
        ));
    }

    let now = Utc::now();
    let tenant_id = Uuid::new_v4();
    let principal_id = Uuid::new_v4();
    let slug = password::unique_slug(&email);
    let display_name = DEFAULT_DISPLAY_NAME;

    let mut tx = db.begin().await?;

    sqlx::query(
        r#"
        INSERT INTO tenants (id, kind, name, slug, status, security_tier, data_region, created_at, updated_at)
        VALUES ($1, 'personal', $2, $3, 'active', 'standard', $4, $5, $5)
        "#,
    )
    .bind(tenant_id)
    .bind("Personal account")
    .bind(slug)
    .bind(data_region.as_deref().unwrap_or("eu"))
    .bind(now)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO principals (id, tenant_id, principal_kind, status, display_name, created_at, updated_at)
        VALUES ($1, $2, 'human', 'active', $3, $4, $4)
        "#,
    )
    .bind(principal_id)
    .bind(tenant_id)
    .bind(Option::<String>::None)
    .bind(now)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO users (
          principal_id, email, password_hash, email_verified_at, status,
          password_last_changed_at, created_at, updated_at
        )
        VALUES ($1, $2, $3, NULL, 'pending_verification', $4, $4, $4)
        "#,
    )
    .bind(principal_id)
    .bind(&email)
    .bind(&password_hash)
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(crate::domains::auth::db::emails::email_constraint_error)?;

    crate::domains::auth::db::registration_enrollment::insert_tx(
        &mut tx,
        principal_id,
        &registration_enrollment_token_hash,
        registration_enrollment_expires_at,
    )
    .await?;

    sqlx::query(
        r#"
        INSERT INTO user_email_addresses (
          principal_id,
          email,
          normalized_email,
          is_primary,
          verified_at,
          created_at,
          updated_at
        )
        VALUES ($1, $2, lower($2), TRUE, NULL, $3, $3)
        "#,
    )
    .bind(principal_id)
    .bind(&email)
    .bind(now)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO tenant_memberships (tenant_id, principal_id, principal_kind, status, source, created_at, updated_at)
        VALUES ($1, $2, 'human', 'active', 'manual', $3, $3)
        "#,
    )
    .bind(tenant_id)
    .bind(principal_id)
    .bind(now)
    .execute(&mut *tx)
    .await?;

    crate::domains::auth::account_registration_event::enqueue_tx(
        &mut tx,
        principal_id,
        marketing_emails_accepted,
        ip.as_deref(),
        now,
    )
    .await?;

    crate::domains::audit::record_event_tx(
        &mut tx,
        crate::domains::audit::AuditRecordInput {
            tenant_id,
            workspace_id: None,
            actor_principal_id: Some(principal_id),
            action: "user.registered",
            target_type: "user",
            target_id: Some(principal_id),
            ip: ip.as_deref(),
            user_agent: user_agent.as_deref(),
            metadata: serde_json::json!({"email": email}),
        },
    )
    .await?;

    tx.commit().await?;

    email_verification::issue_verification_email_tx(
        redis,
        config,
        principal_id,
        &email,
        &display_name,
        &verification_token,
    )
    .await?;

    Ok((principal_id, now))
}
