use crate::grpc::pb::nvbes::enterprise::v1 as enterprise;
use sqlx::Row;
use tonic::Status;
use uuid::Uuid;

use crate::grpc::service_status::sql_status;

pub async fn upsert_break_glass(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
    principal_id: Uuid,
    actor_id: Uuid,
    procedure_reference: &str,
    reason: &str,
) -> Result<(), Status> {
    sqlx::query(
        r#"
        INSERT INTO tenant_break_glass_accounts (
          tenant_id, principal_id, procedure_reference, reason, created_by_principal_id
        )
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (tenant_id, principal_id)
        DO UPDATE SET
          procedure_reference = EXCLUDED.procedure_reference,
          reason = EXCLUDED.reason,
          created_by_principal_id = EXCLUDED.created_by_principal_id,
          revoked_by_principal_id = NULL,
          revoked_reason = NULL,
          revoked_at = NULL,
          updated_at = NOW()
        "#,
    )
    .bind(tenant_id)
    .bind(principal_id)
    .bind(procedure_reference)
    .bind(reason)
    .bind(actor_id)
    .execute(db)
    .await
    .map_err(sql_status)?;
    Ok(())
}

pub async fn revoke_break_glass(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
    principal_id: Uuid,
    actor_id: Uuid,
    reason: &str,
) -> Result<u64, Status> {
    sqlx::query(
        r#"
        UPDATE tenant_break_glass_accounts
        SET revoked_by_principal_id = $3,
            revoked_reason = $4,
            revoked_at = NOW(),
            updated_at = NOW()
        WHERE tenant_id = $1 AND principal_id = $2 AND revoked_at IS NULL
        "#,
    )
    .bind(tenant_id)
    .bind(principal_id)
    .bind(actor_id)
    .bind(reason)
    .execute(db)
    .await
    .map(|result| result.rows_affected())
    .map_err(sql_status)
}

pub async fn list_break_glass_accounts(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
    principal_ids: &[Uuid],
) -> Result<Vec<enterprise::BreakGlassAccount>, Status> {
    let rows = sqlx::query(
        r#"
        SELECT tenant_id, principal_id, procedure_reference, reason, created_at, last_used_at
        FROM tenant_break_glass_accounts
        WHERE tenant_id = $1
          AND revoked_at IS NULL
          AND (cardinality($2::uuid[]) = 0 OR principal_id = ANY($2))
        ORDER BY created_at DESC
        "#,
    )
    .bind(tenant_id)
    .bind(principal_ids)
    .fetch_all(db)
    .await
    .map_err(sql_status)?;

    Ok(rows
        .into_iter()
        .map(|row| {
            let last_used_at: Option<chrono::DateTime<chrono::Utc>> = row.get("last_used_at");
            enterprise::BreakGlassAccount {
                tenant_id: row.get::<Uuid, _>("tenant_id").to_string(),
                principal_id: row.get::<Uuid, _>("principal_id").to_string(),
                procedure_reference: row.get("procedure_reference"),
                reason: row.get("reason"),
                created_at: row
                    .get::<chrono::DateTime<chrono::Utc>, _>("created_at")
                    .to_rfc3339(),
                last_used_at: last_used_at
                    .map(|value| value.to_rfc3339())
                    .unwrap_or_default(),
            }
        })
        .collect())
}
