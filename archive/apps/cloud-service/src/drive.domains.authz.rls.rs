use sqlx::{PgPool, Postgres, Transaction};

use super::types::WorkspaceAccess;

pub async fn begin_workspace_transaction<'a>(
    db: &'a PgPool,
    access: &WorkspaceAccess,
) -> Result<Transaction<'a, Postgres>, sqlx::Error> {
    let mut tx = db.begin().await?;
    nvbes_tenancy::set_transaction_rls_context(
        &mut tx,
        nvbes_tenancy::RlsContext {
            principal_id: Some(access.auth.principal_id),
            user_id: Some(access.auth.user_id),
            tenant_id: access.tenant_id.or(access.auth.tenant_id),
            workspace_id: Some(access.workspace_id),
        },
    )
    .await?;
    Ok(tx)
}
