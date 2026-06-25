use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::error::AppError;

#[derive(Debug, Serialize)]
pub(crate) struct OperatorRoleDistribution {
    pub(crate) role: String,
    pub(crate) active_count: i64,
}

#[derive(Debug, Serialize)]
pub(crate) struct OperatorGrant {
    pub(crate) principal_id: Uuid,
    pub(crate) email: Option<String>,
    pub(crate) display_name: Option<String>,
    pub(crate) role: String,
    pub(crate) status: String,
    pub(crate) granted_at: DateTime<Utc>,
    pub(crate) revoked_at: Option<DateTime<Utc>>,
    pub(crate) reason: Option<String>,
}

#[derive(Debug)]
pub(crate) struct OperatorGrantSnapshot {
    pub(crate) active_count: i64,
    pub(crate) revoked_count: i64,
    pub(crate) role_distribution: Vec<OperatorRoleDistribution>,
    pub(crate) grants: Vec<OperatorGrant>,
}

pub(crate) async fn load_operator_grants(db: &PgPool) -> Result<OperatorGrantSnapshot, AppError> {
    if !operator_grants_table_exists(db).await? {
        return Ok(OperatorGrantSnapshot {
            active_count: 0,
            revoked_count: 0,
            role_distribution: Vec::new(),
            grants: Vec::new(),
        });
    }

    let metrics = sqlx::query(
        r#"
        SELECT
          COUNT(*) FILTER (WHERE status = 'active') AS active_count,
          COUNT(*) FILTER (WHERE status = 'revoked') AS revoked_count
        FROM internal_admin_operator_grants
        "#,
    )
    .fetch_one(db)
    .await?;

    Ok(OperatorGrantSnapshot {
        active_count: metrics.get("active_count"),
        revoked_count: metrics.get("revoked_count"),
        role_distribution: load_role_distribution(db).await?,
        grants: load_recent_operator_grants(db).await?,
    })
}

async fn load_role_distribution(db: &PgPool) -> Result<Vec<OperatorRoleDistribution>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT role, COUNT(*) AS active_count
        FROM internal_admin_operator_grants
        WHERE status = 'active'
        GROUP BY role
        ORDER BY active_count DESC, role ASC
        "#,
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| OperatorRoleDistribution {
            role: row.get("role"),
            active_count: row.get("active_count"),
        })
        .collect())
}

async fn load_recent_operator_grants(db: &PgPool) -> Result<Vec<OperatorGrant>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT g.principal_id, u.email, p.display_name, g.role, g.status,
          g.granted_at, g.revoked_at, g.reason
        FROM internal_admin_operator_grants g
        JOIN principals p ON p.id = g.principal_id
        LEFT JOIN users u ON u.principal_id = g.principal_id
        ORDER BY
          CASE WHEN g.status = 'active' THEN 0 ELSE 1 END,
          COALESCE(g.revoked_at, g.granted_at) DESC
        LIMIT 12
        "#,
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| OperatorGrant {
            principal_id: row.get("principal_id"),
            email: row.get("email"),
            display_name: row.get("display_name"),
            role: row.get("role"),
            status: row.get("status"),
            granted_at: row.get("granted_at"),
            revoked_at: row.get("revoked_at"),
            reason: row.get("reason"),
        })
        .collect())
}

async fn operator_grants_table_exists(db: &PgPool) -> Result<bool, AppError> {
    Ok(sqlx::query_scalar::<_, bool>(
        "SELECT to_regclass('public.internal_admin_operator_grants') IS NOT NULL",
    )
    .fetch_one(db)
    .await?)
}
