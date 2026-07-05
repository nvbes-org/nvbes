use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::http::error::AppError;

pub async fn upsert_break_glass_account(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    principal_id: Uuid,
    actor_id: Uuid,
    procedure_reference: &str,
    reason: &str,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO tenant_break_glass_accounts (
          tenant_id,
          principal_id,
          procedure_reference,
          reason,
          created_by_principal_id
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
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub async fn revoke_break_glass_account(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    principal_id: Uuid,
    actor_id: Uuid,
    reason: &str,
) -> Result<u64, AppError> {
    let result = sqlx::query(
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
    .execute(&mut **tx)
    .await?;
    Ok(result.rows_affected())
}

pub async fn touch_break_glass_account(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    principal_id: Uuid,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE tenant_break_glass_accounts
        SET last_used_at = NOW(), updated_at = NOW()
        WHERE tenant_id = $1 AND principal_id = $2 AND revoked_at IS NULL
        "#,
    )
    .bind(tenant_id)
    .bind(principal_id)
    .execute(&mut **tx)
    .await?;
    Ok(())
}
