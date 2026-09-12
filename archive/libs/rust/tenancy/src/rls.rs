use sqlx::{Postgres, Transaction};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Default)]
pub struct RlsContext {
    pub principal_id: Option<Uuid>,
    pub user_id: Option<Uuid>,
    pub tenant_id: Option<Uuid>,
    pub workspace_id: Option<Uuid>,
}

pub async fn set_transaction_rls_context(
    tx: &mut Transaction<'_, Postgres>,
    context: RlsContext,
) -> Result<(), sqlx::Error> {
    set_local_uuid(tx, "nvbes.principal_id", context.principal_id).await?;
    set_local_uuid(tx, "nvbes.user_id", context.user_id).await?;
    set_local_uuid(tx, "nvbes.tenant_id", context.tenant_id).await?;
    set_local_uuid(tx, "nvbes.workspace_id", context.workspace_id).await?;
    Ok(())
}

async fn set_local_uuid(
    tx: &mut Transaction<'_, Postgres>,
    setting: &'static str,
    value: Option<Uuid>,
) -> Result<(), sqlx::Error> {
    let Some(value) = value else {
        return Ok(());
    };

    sqlx::query("SELECT set_config($1, $2, true)")
        .bind(setting)
        .bind(value.to_string())
        .execute(&mut **tx)
        .await?;

    Ok(())
}
