use sqlx::PgPool;

use crate::domains::service_accounts::routes::tests::seed::ServiceAccountRouteFixture;

pub(super) async fn cleanup(
    pool: &PgPool,
    redis: &nvbes_redis::RedisPool,
    fixture: &ServiceAccountRouteFixture,
) {
    let _ =
        nvbes_redis::session::clear_user_sessions(redis, &fixture.admin_principal_id.to_string())
            .await;
    sqlx::query("DELETE FROM oauth_client_policies WHERE client_id = $1")
        .bind(fixture.client_uuid)
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM oauth_clients WHERE id = $1")
        .bind(fixture.client_uuid)
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM service_accounts WHERE principal_id = $1")
        .bind(fixture.service_principal_id)
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM workspace_memberships WHERE principal_id = $1")
        .bind(fixture.service_principal_id)
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM workspace_memberships WHERE principal_id = $1")
        .bind(fixture.admin_principal_id)
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM users WHERE principal_id = $1")
        .bind(fixture.admin_principal_id)
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM principals WHERE id = $1")
        .bind(fixture.service_principal_id)
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM principals WHERE id = $1")
        .bind(fixture.admin_principal_id)
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM workspaces WHERE id = $1")
        .bind(fixture.workspace_id)
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM tenants WHERE id = $1")
        .bind(fixture.tenant_id)
        .execute(pool)
        .await
        .ok();
}
