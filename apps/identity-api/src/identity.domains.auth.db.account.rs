use chrono::{DateTime, NaiveDate, Utc};
use nvbes_core::config::AppConfig;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domains::auth::{email_verification, password, types::derive_display_name};
use crate::domains::billing;
use crate::http::error::AppError;

#[expect(
    clippy::too_many_arguments,
    reason = "Account creation keeps registration, workspace, and audit inputs explicit."
)]
pub async fn create_user_account(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    config: &AppConfig,
    email: String,
    firstname: Option<String>,
    lastname: Option<String>,
    username: String,
    birthdate: Option<NaiveDate>,
    region: Option<String>,
    data_region: Option<String>,
    workspace_name: String,
    password_hash: String,
    verification_token: String,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<(Uuid, Uuid, DateTime<Utc>), AppError> {
    let now = Utc::now();
    let tenant_id = Uuid::new_v4();
    let principal_id = Uuid::new_v4();
    let workspace_id = Uuid::new_v4();
    let slug = password::unique_slug(&email);
    let display_name =
        derive_display_name(firstname.as_deref(), lastname.as_deref(), Some(&username));

    let mut tx = db.begin().await?;
    billing::db::ensure_plan_seeded(&mut tx).await?;

    sqlx::query(
        r#"
        INSERT INTO tenants (id, kind, name, slug, status, security_tier, created_at, updated_at)
        VALUES ($1, 'personal', $2, $3, 'active', 'standard', $4, $4)
        "#,
    )
    .bind(tenant_id)
    .bind(&username)
    .bind(slug)
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
        INSERT INTO users (principal_id, email, firstname, lastname, username, birthdate, region, password_hash, email_verified_at, status, password_last_changed_at, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NULL, 'pending_verification', $9, $9, $9)
        "#,
    )
    .bind(principal_id)
    .bind(&email)
    .bind(&firstname)
    .bind(&lastname)
    .bind(Some(username))
    .bind(birthdate)
    .bind(&region)
    .bind(&password_hash)
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

    sqlx::query(
        r#"
        INSERT INTO workspaces (id, tenant_id, organization_id, name, workspace_type, plan_code, trial_ends_at, data_region, jurisdiction, created_at, updated_at)
        VALUES ($1, $2, NULL, $3, 'personal', 'solo_pro', $4, $5::data_region, $6::legal_jurisdiction, $7, $7)
        "#,
    )
    .bind(workspace_id)
    .bind(tenant_id)
    .bind(&workspace_name)
    .bind(now + chrono::Duration::days(14))
    .bind(data_region.as_deref().unwrap_or("eu"))
    .bind(match data_region.as_deref() {
        Some("us") => "ccpa",
        Some("ch") => "nfdap",
        _ => "gdpr",
    })
    .bind(now)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO workspace_policies (
          workspace_id,
          member_can_create_share_links,
          require_admin_approval_for_member_share,
          default_share_link_ttl_days,
          max_share_link_ttl_days,
          required_acr,
          created_at,
          updated_at
        )
        VALUES ($1, FALSE, TRUE, 7, 30, 'aal1', $2, $2)
        "#,
    )
    .bind(workspace_id)
    .bind(now)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO workspace_memberships (workspace_id, principal_id, role, status, source, created_at, updated_at)
        VALUES ($1, $2, 'owner', 'active', 'manual', $3, $3)
        "#,
    )
    .bind(workspace_id)
    .bind(principal_id)
    .bind(now)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO audit_events (
          tenant_id, workspace_id, actor_principal_id, action, target_type, target_id, ip, user_agent, metadata, event_hash
        )
        VALUES ($1, $2, $3, 'user.registered', 'user', $3, $4::inet, $5, $6, $7)
        "#,
    )
    .bind(tenant_id)
    .bind(workspace_id)
    .bind(principal_id)
    .bind(ip.as_deref())
    .bind(user_agent.as_deref())
    .bind(serde_json::json!({"email": email}))
    .bind(password::token_hash(&format!("{principal_id}:{tenant_id}:{workspace_id}")))
    .execute(&mut *tx)
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

    Ok((principal_id, workspace_id, now))
}
