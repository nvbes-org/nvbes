use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::http::error::AppError;

pub async fn set_session_policy(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    admin_session_ttl_hours: i64,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO tenant_policies (tenant_id, admin_session_ttl_hours)
        VALUES ($1, $2)
        ON CONFLICT (tenant_id)
        DO UPDATE SET admin_session_ttl_hours = EXCLUDED.admin_session_ttl_hours,
                      updated_at = NOW()
        "#,
    )
    .bind(tenant_id)
    .bind(admin_session_ttl_hours as i32)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub async fn set_mfa_policy(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    policy: &str,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE tenants
        SET mfa_policy = $2,
            updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(tenant_id)
    .bind(policy)
    .execute(&mut **tx)
    .await?;
    Ok(())
}
