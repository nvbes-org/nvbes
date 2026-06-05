use chrono::{DateTime, NaiveDate, Utc};
use nvbes_core::config::AppConfig;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use super::{mfa, password, types::*};
use crate::domains::billing;
use crate::http::error::AppError;

#[derive(Debug)]
pub struct UserRecord {
    pub principal_id: Uuid,
    pub email: String,
    pub display_name: String,
    pub status: String,
    pub firstname: Option<String>,
    pub lastname: Option<String>,
    pub username: Option<String>,
    pub birthdate: Option<NaiveDate>,
    pub region: Option<String>,
    pub password_hash: Option<String>,
    pub email_verified_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

pub async fn fetch_user_record(db: &PgPool, principal_id: Uuid) -> Result<UserRecord, AppError> {
    let row = sqlx::query(
        r#"
        SELECT
          u.principal_id,
          u.email,
          u.firstname,
          u.lastname,
          u.username,
          u.birthdate,
          u.region,
          u.password_hash,
          u.email_verified_at,
          u.created_at,
          u.status::text AS status
        FROM users u
        WHERE u.principal_id = $1
        LIMIT 1
        "#,
    )
    .bind(principal_id)
    .fetch_one(db)
    .await?;

    let firstname: Option<String> = row.get("firstname");
    let lastname: Option<String> = row.get("lastname");
    let username: Option<String> = row.get("username");
    let display_name = derive_display_name(
        firstname.as_deref(),
        lastname.as_deref(),
        username.as_deref(),
    );

    Ok(UserRecord {
        principal_id: row.get("principal_id"),
        email: row.get("email"),
        display_name,
        status: row.get("status"),
        firstname,
        lastname,
        username,
        birthdate: row.get("birthdate"),
        region: row.get("region"),
        password_hash: row.get("password_hash"),
        email_verified_at: row.get("email_verified_at"),
        created_at: row.get("created_at"),
    })
}

pub async fn fetch_user_view(db: &PgPool, principal_id: Uuid) -> Result<UserView, AppError> {
    let user = fetch_user_record(db, principal_id).await?;
    Ok(UserView {
        id: user.principal_id,
        email: user.email,
        display_name: user.display_name,
        firstname: user.firstname,
        lastname: user.lastname,
        username: user.username,
        birthdate: user.birthdate,
        region: user.region,
        email_verified: user.email_verified_at.is_some(),
        mfa_enabled: mfa::has_active_factor(db, principal_id).await?,
        created_at: user.created_at,
    })
}
pub async fn update_user_profile(
    db: &PgPool,
    principal_id: Uuid,
    input: &UpdateProfileInput,
) -> Result<UserView, AppError> {
    let existing = fetch_user_record(db, principal_id).await?;

    let firstname = input.firstname.clone().or(existing.firstname);
    let lastname = input.lastname.clone().or(existing.lastname);
    let username = input.username.clone().or(existing.username);
    let birthdate = input.birthdate.or(existing.birthdate);
    let region = input.region.clone().or(existing.region);
    let display_name = derive_display_name(
        firstname.as_deref(),
        lastname.as_deref(),
        username.as_deref(),
    );

    if let Some(ref new_username) = input.username {
        if new_username.trim().is_empty() {
            return Err(AppError::bad_request(
                "validation_failed",
                "Username cannot be empty.",
            ));
        }
        if new_username.len() > 100 {
            return Err(AppError::bad_request(
                "validation_failed",
                "Username must be 100 characters or fewer.",
            ));
        }
    }

    let result = sqlx::query(
        r#"
        UPDATE users
        SET firstname = $2, lastname = $3, username = $4, birthdate = $5, region = $6, updated_at = NOW()
        WHERE principal_id = $1
        RETURNING email, email_verified_at, created_at
        "#,
    )
    .bind(principal_id)
    .bind(&firstname)
    .bind(&lastname)
    .bind(&username)
    .bind(birthdate)
    .bind(&region)
    .fetch_one(db)
    .await
    .map_err(|e| {
        if let sqlx::Error::Database(ref db_err) = e {
            if db_err.constraint() == Some("idx_users_username") {
                AppError::conflict(
                    "username_taken",
                    "This username is already taken.",
                )
            } else {
                AppError::internal("database_error", "Failed to update profile.")
            }
        } else {
            e.into()
        }
    })?;

    use sqlx::Row;
    Ok(UserView {
        id: existing.principal_id,
        email: result.get("email"),
        display_name,
        firstname,
        lastname,
        username,
        birthdate,
        region,
        email_verified: result
            .get::<Option<DateTime<Utc>>, _>("email_verified_at")
            .is_some(),
        mfa_enabled: super::mfa::has_active_factor(db, principal_id).await?,
        created_at: result.get("created_at"),
    })
}
pub async fn fetch_user_preferences(
    db: &PgPool,
    principal_id: Uuid,
) -> Result<UserPreferences, AppError> {
    let row = sqlx::query("SELECT preferences FROM users WHERE principal_id = $1")
        .bind(principal_id)
        .fetch_optional(db)
        .await?;

    let prefs: serde_json::Value = row
        .as_ref()
        .and_then(|r| sqlx::Row::try_get::<serde_json::Value, _>(r, "preferences").ok())
        .unwrap_or(serde_json::json!({}));

    Ok(UserPreferences {
        theme: prefs["theme"].as_str().unwrap_or("system").to_string(),
        language: prefs["language"].as_str().unwrap_or("en").to_string(),
        skip_password: prefs["skip_password"].as_bool().unwrap_or(false),
    })
}

pub async fn update_user_preferences(
    db: &PgPool,
    principal_id: Uuid,
    input: &UserPreferences,
) -> Result<UserPreferences, AppError> {
    let json_value = serde_json::json!({
        "theme": input.theme,
        "language": input.language,
        "skip_password": input.skip_password,
    });

    sqlx::query("UPDATE users SET preferences = $2, updated_at = NOW() WHERE principal_id = $1")
        .bind(principal_id)
        .bind(&json_value)
        .execute(db)
        .await?;

    Ok(input.clone())
}

pub async fn fetch_user_notifications(
    db: &PgPool,
    principal_id: Uuid,
) -> Result<UserNotifications, AppError> {
    let row = sqlx::query("SELECT notifications FROM users WHERE principal_id = $1")
        .bind(principal_id)
        .fetch_optional(db)
        .await?;

    let notifs: serde_json::Value = row
        .as_ref()
        .and_then(|r| sqlx::Row::try_get::<serde_json::Value, _>(r, "notifications").ok())
        .unwrap_or(serde_json::json!({}));

    Ok(UserNotifications {
        email: notifs["email"].as_bool().unwrap_or(true),
        push: notifs["push"].as_bool().unwrap_or(true),
        in_app: notifs["in_app"].as_bool().unwrap_or(true),
    })
}

pub async fn update_user_notifications(
    db: &PgPool,
    principal_id: Uuid,
    input: &UserNotifications,
) -> Result<UserNotifications, AppError> {
    let json_value = serde_json::json!({
        "email": input.email,
        "push": input.push,
        "in_app": input.in_app,
    });

    sqlx::query("UPDATE users SET notifications = $2, updated_at = NOW() WHERE principal_id = $1")
        .bind(principal_id)
        .bind(&json_value)
        .execute(db)
        .await?;

    Ok(input.clone())
}

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

    super::email_verification::issue_verification_email_tx(
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

pub fn map_factor_view(row: sqlx::postgres::PgRow) -> MfaFactorView {
    MfaFactorView {
        id: row.get("id"),
        factor_type: row.get("factor_type"),
        kind: row.try_get::<Option<String>, _>("kind").ok().flatten(),
        status: row.get("status"),
        label: row.get("label"),
        created_at: row.get("created_at"),
        confirmed_at: row.get("confirmed_at"),
        last_used_at: row.get("last_used_at"),
    }
}
