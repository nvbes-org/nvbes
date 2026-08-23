use crate::{
    grpc::pb::nvbes::developer::v1 as developer,
    test_support::{
        cleanup_tenant, has_developer_contract_schema, seed_developer_fixture, test_pool,
    },
};

#[tokio::test]
async fn developer_roles_are_tenant_and_principal_scoped() {
    let pool = test_pool();
    if !has_developer_rbac_schema(&pool).await {
        eprintln!("skipping test: local database is missing Developer RBAC schema");
        return;
    }

    let fixture = seed_developer_fixture(&pool, "rbac").await;
    sqlx::query(
        r#"
        INSERT INTO developer_role_assignments (tenant_id, principal_id, role, created_at)
        VALUES
          ($1, $2, 'log_viewer', $3),
          ($1, $2, 'app_manager', $3)
        "#,
    )
    .bind(fixture.tenant_id)
    .bind(fixture.principal_id)
    .bind(fixture.now)
    .execute(&pool)
    .await
    .expect("developer roles should be seeded");

    let roles = super::list_developer_roles(
        &pool,
        developer::ListDeveloperRolesRequest {
            context: None,
            tenant_id: fixture.tenant_id.to_string(),
            principal_id: fixture.principal_id.to_string(),
        },
    )
    .await
    .expect("roles should be listed");

    assert_eq!(roles.roles, vec!["app_manager", "log_viewer"]);

    cleanup_tenant(&pool, fixture.tenant_id).await;
}

async fn has_developer_rbac_schema(pool: &sqlx::PgPool) -> bool {
    has_developer_contract_schema(pool).await
        && sqlx::query_scalar::<_, bool>(
            "SELECT to_regclass('public.developer_role_assignments') IS NOT NULL",
        )
        .fetch_one(pool)
        .await
        .unwrap_or(false)
}
