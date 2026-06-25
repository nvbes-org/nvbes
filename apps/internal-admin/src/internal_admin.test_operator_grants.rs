use sqlx::PgPool;
use uuid::Uuid;

pub(crate) async fn operator_grants_schema_exists(pool: &PgPool) -> bool {
    sqlx::query_scalar::<_, bool>(
        "SELECT to_regclass('public.internal_admin_operator_grants') IS NOT NULL",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false)
}

pub(crate) async fn grant_active_operator_role(pool: &PgPool, principal_id: Uuid, role: &str) {
    sqlx::query(
        "INSERT INTO internal_admin_operator_grants (
          principal_id, role, status, granted_by_principal_id, reason
        ) VALUES ($1, $2, 'active', $1, 'test bootstrap')
        ON CONFLICT (principal_id, role) DO UPDATE
        SET status = 'active',
            granted_by_principal_id = EXCLUDED.granted_by_principal_id,
            revoked_at = NULL,
            reason = EXCLUDED.reason",
    )
    .bind(principal_id)
    .bind(role)
    .execute(pool)
    .await
    .expect("operator grant should insert");
}
