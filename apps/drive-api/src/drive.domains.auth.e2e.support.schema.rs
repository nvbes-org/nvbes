use sqlx::postgres::PgPool;
use uuid::Uuid;

pub(crate) async fn db_supports_current_schema(pool: &PgPool) -> bool {
    let identity_oauth = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
          SELECT 1
          FROM information_schema.columns
          WHERE table_name = 'oauth_clients'
            AND column_name = 'client_assertion_required'
        )
        "#,
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);

    let drive_workspace = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
          SELECT 1
          FROM information_schema.columns
          WHERE table_name = 'workspaces'
            AND column_name = 'tenant_id'
        )
        "#,
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);

    let drive_users_have_id = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
          SELECT 1
          FROM information_schema.columns
          WHERE table_name = 'users'
            AND column_name = 'id'
        )
        "#,
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);

    let drive_created_by_principal = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
          SELECT 1
          FROM information_schema.columns
          WHERE table_name = 'storage_objects'
            AND column_name = 'created_by_principal_id'
        )
        "#,
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);

    let drive_share_links_created_by_principal = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
          SELECT 1
          FROM information_schema.columns
          WHERE table_name = 'share_links'
            AND column_name = 'created_by_principal_id'
        )
        "#,
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);

    let drive_upload_sessions_created_by_principal = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
          SELECT 1
          FROM information_schema.columns
          WHERE table_name = 'upload_sessions'
            AND column_name = 'created_by_principal_id'
        )
        "#,
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);

    identity_oauth
        && drive_workspace
        && drive_users_have_id
        && drive_created_by_principal
        && drive_share_links_created_by_principal
        && drive_upload_sessions_created_by_principal
}

pub(crate) async fn cleanup(pool: &PgPool, tenant_id: Uuid, owner_email: &str) {
    sqlx::query("DELETE FROM users WHERE email = $1")
        .bind(owner_email)
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM tenants WHERE id = $1")
        .bind(tenant_id)
        .execute(pool)
        .await
        .ok();
}
