#[path = "drive.domains.public_api.db.request_logs.rs"]
mod request_logs;

pub use request_logs::insert_api_request_log;

use super::types::{ApiKeyMigrationTargetView, ApiKeyView, AuditEventInsert, RateLimitView};
use sqlx::{PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

pub async fn list_api_keys(
    db: &PgPool,
    workspace_id: Uuid,
) -> Result<Vec<ApiKeyView>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT
          id,
          workspace_id,
          name,
          key_prefix,
          scopes,
          status::text AS status,
          created_by,
          created_by_principal_id,
          created_at,
          expires_at,
          last_used_at,
          last_used_ip::text AS last_used_ip,
          http_signature_public_key
        FROM api_keys
        WHERE workspace_id = $1
        ORDER BY created_at DESC
        "#,
    )
    .bind(workspace_id)
    .fetch_all(db)
    .await?;

    Ok(rows.into_iter().map(api_key_view).collect())
}

pub async fn update_api_key_status(
    tx: &mut Transaction<'_, Postgres>,
    api_key_id: Uuid,
    workspace_id: Uuid,
    status: &str,
) -> Result<Option<ApiKeyView>, sqlx::Error> {
    let row = sqlx::query(
        r#"
        UPDATE api_keys
        SET status = $3::api_key_status
        WHERE id = $1
          AND workspace_id = $2
        RETURNING
          id,
          workspace_id,
          name,
          key_prefix,
          scopes,
          status::text AS status,
          created_by,
          created_by_principal_id,
          created_at,
          expires_at,
          last_used_at,
          last_used_ip::text AS last_used_ip,
          http_signature_public_key
        "#,
    )
    .bind(api_key_id)
    .bind(workspace_id)
    .bind(status)
    .fetch_optional(&mut **tx)
    .await?;

    Ok(row.map(api_key_view))
}

pub async fn get_api_key_by_hash(
    db: &PgPool,
    hash: &str,
) -> Result<Option<sqlx::postgres::PgRow>, sqlx::Error> {
    sqlx::query(
        r#"
        SELECT
          ak.id,
          ak.workspace_id,
          ak.key_prefix,
          ak.scopes,
          ak.status::text AS status,
          ak.created_by,
          ak.created_by_principal_id,
          ak.expires_at,
          ak.http_signature_public_key,
          w.deleted_at,
          p.code AS plan_code
        FROM api_keys ak
        INNER JOIN workspaces w ON w.id = ak.workspace_id
        INNER JOIN plans p ON p.id = w.plan_id
        WHERE ak.key_hash = $1
        "#,
    )
    .bind(hash)
    .fetch_optional(db)
    .await
}

pub async fn update_last_used(
    db: &PgPool,
    api_key_id: Uuid,
    ip: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE api_keys SET last_used_at = NOW(), last_used_ip = $2::inet WHERE id = $1")
        .bind(api_key_id)
        .bind(ip)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn consume_api_key_nonce(
    db: &PgPool,
    workspace_id: Uuid,
    api_key_id: Uuid,
    nonce: &str,
    timestamp: chrono::DateTime<chrono::Utc>,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        r#"
        INSERT INTO api_key_nonces (workspace_id, api_key_id, nonce, timestamp)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (api_key_id, nonce) DO NOTHING
        "#,
    )
    .bind(workspace_id)
    .bind(api_key_id)
    .bind(nonce)
    .bind(timestamp)
    .execute(db)
    .await?;
    Ok(result.rows_affected() == 1)
}

pub async fn get_workspace_for_api(
    db: &PgPool,
    workspace_id: Uuid,
) -> Result<sqlx::postgres::PgRow, sqlx::Error> {
    sqlx::query(
        r#"
        SELECT w.id, w.name, p.code AS plan_code
        FROM workspaces w
        INNER JOIN plans p ON p.id = w.plan_id
        WHERE w.id = $1
        "#,
    )
    .bind(workspace_id)
    .fetch_one(db)
    .await
}

pub async fn insert_audit_event_tx(
    tx: &mut Transaction<'_, Postgres>,
    input: AuditEventInsert<'_>,
) -> Result<(), sqlx::Error> {
    let metadata = crate::domains::audit::geo::enrich_audit_metadata_tx(
        tx,
        input.workspace_id,
        input.ip,
        input.action,
        input.metadata,
    )
    .await?;

    sqlx::query(
        r#"
        INSERT INTO audit_events (
          workspace_id,
          actor_user_id,
          actor_principal_id,
          action,
          target_type,
          target_id,
          ip,
          user_agent,
          metadata
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7::inet, $8, $9)
        "#,
    )
    .bind(input.workspace_id)
    .bind(input.actor_user_id)
    .bind(input.actor_principal_id)
    .bind(input.action)
    .bind(input.target_type)
    .bind(input.target_id)
    .bind(input.ip)
    .bind(input.user_agent)
    .bind(sqlx::types::Json(metadata))
    .execute(&mut **tx)
    .await?;
    Ok(())
}

fn api_key_view(row: sqlx::postgres::PgRow) -> ApiKeyView {
    ApiKeyView {
        id: row.get("id"),
        workspace_id: row.get("workspace_id"),
        name: row.get("name"),
        key_prefix: row.get("key_prefix"),
        scopes: row.get("scopes"),
        status: row.get("status"),
        created_by: row.get("created_by"),
        created_by_principal_id: row.get("created_by_principal_id"),
        created_at: row.get("created_at"),
        expires_at: row.get("expires_at"),
        last_used_at: row.get("last_used_at"),
        last_used_ip: row.get("last_used_ip"),
        http_signature_public_key: row.get("http_signature_public_key"),
        deprecated: true,
        sunset_at: Some(
            chrono::TimeZone::with_ymd_and_hms(&chrono::Utc, 2026, 12, 31, 23, 59, 59)
                .single()
                .expect("valid sunset date"),
        ),
        migration_target: ApiKeyMigrationTargetView {
            workspace_id: row.get("workspace_id"),
            identity_management_path: format!(
                "/workspaces/{}/service-accounts",
                row.get::<Uuid, _>("workspace_id")
            ),
            recommended_flow: "service_account_oauth_client".to_string(),
        },
    }
}

pub fn rate_limits_for_plan(plan_code: &str) -> RateLimitView {
    match plan_code {
        "solo_pro" => RateLimitView {
            requests_per_minute: 60,
            requests_per_day: 1_000,
        },
        "team_plus" => RateLimitView {
            requests_per_minute: 600,
            requests_per_day: 100_000,
        },
        "team" => RateLimitView {
            requests_per_minute: 300,
            requests_per_day: 20_000,
        },
        _ => RateLimitView {
            requests_per_minute: 30,
            requests_per_day: 500,
        },
    }
}
