use crate::{
    grpc::{pb::nvbes::enterprise::v1 as enterprise, user_access},
    test_support::{
        cleanup_tenant, has_enterprise_contract_schema, seed_enterprise_fixture, test_pool,
    },
};

#[tokio::test]
async fn user_access_changes_round_trip_through_enterprise_store() {
    let pool = test_pool();
    if !has_enterprise_contract_schema(&pool).await {
        eprintln!("skipping test: local database is missing Enterprise contract schema");
        return;
    }

    let fixture = seed_enterprise_fixture(&pool, "user-access").await;
    let (workspace_a, workspace_b) = seed_user_access_memberships(&pool, &fixture).await;
    let update = user_access::update_user_access(
        &pool,
        fixture.tenant_id,
        fixture.actor_id,
        enterprise::UpdateUserAccessRequest {
            context: None,
            tenant_id: fixture.tenant_id.to_string(),
            target_principal_id: fixture.principal_id.to_string(),
            role: "viewer".to_string(),
            workspace_ids: vec![workspace_a.to_string()],
            scope: "tenant".to_string(),
            organization_id: String::new(),
        },
    )
    .await
    .expect("user access should update");

    assert_eq!(update.status, "active");
    assert_eq!(update.workspace_ids, vec![workspace_a.to_string()]);
    assert_workspace_membership(&pool, workspace_a, fixture.principal_id, "viewer", "active").await;
    assert_workspace_membership(
        &pool,
        workspace_b,
        fixture.principal_id,
        "member",
        "removed",
    )
    .await;

    user_access::suspend_user_access(
        &pool,
        fixture.tenant_id,
        fixture.actor_id,
        enterprise::SuspendUserAccessRequest {
            context: None,
            tenant_id: fixture.tenant_id.to_string(),
            target_principal_id: fixture.principal_id.to_string(),
            reason: "offboarding".to_string(),
            scope: "tenant".to_string(),
            organization_id: String::new(),
        },
    )
    .await
    .expect("user access should suspend");
    assert_workspace_membership(
        &pool,
        workspace_a,
        fixture.principal_id,
        "viewer",
        "suspended",
    )
    .await;

    user_access::reactivate_user_access(
        &pool,
        fixture.tenant_id,
        fixture.actor_id,
        enterprise::ReactivateUserAccessRequest {
            context: None,
            tenant_id: fixture.tenant_id.to_string(),
            target_principal_id: fixture.principal_id.to_string(),
            reason: "return".to_string(),
            workspace_ids: vec![workspace_a.to_string()],
            scope: "tenant".to_string(),
            organization_id: String::new(),
        },
    )
    .await
    .expect("user access should reactivate");
    assert_workspace_membership(&pool, workspace_a, fixture.principal_id, "viewer", "active").await;

    let audit_count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM audit_events
        WHERE tenant_id = $1
          AND target_id = $2
          AND action IN (
            'enterprise.member.access_updated',
            'enterprise.member.suspended',
            'enterprise.member.reactivated'
          )
        "#,
    )
    .bind(fixture.tenant_id)
    .bind(fixture.principal_id)
    .fetch_one(&pool)
    .await
    .expect("audit events should be readable");
    assert_eq!(audit_count, 3);

    cleanup_tenant(&pool, fixture.tenant_id).await;
}

async fn seed_user_access_memberships(
    pool: &sqlx::PgPool,
    fixture: &crate::test_support::EnterpriseFixture,
) -> (uuid::Uuid, uuid::Uuid) {
    let workspace_a = uuid::Uuid::new_v4();
    let workspace_b = uuid::Uuid::new_v4();
    for workspace_id in [workspace_a, workspace_b] {
        sqlx::query(
            r#"
            INSERT INTO workspaces (
              id, tenant_id, name, workspace_type, owner_user_id, plan_code, created_at, updated_at
            )
            VALUES ($1, $2, 'Enterprise user access workspace', 'team', $3, 'team', $4, $4)
            "#,
        )
        .bind(workspace_id)
        .bind(fixture.tenant_id)
        .bind(fixture.actor_id)
        .bind(fixture.now)
        .execute(pool)
        .await
        .expect("workspace should be seeded");

        sqlx::query(
            r#"
            INSERT INTO workspace_memberships (
              workspace_id, principal_id, role, status, source, created_at, updated_at
            )
            VALUES
              ($1, $2, 'owner', 'active', 'manual', $4, $4),
              ($1, $3, 'member', 'active', 'manual', $4, $4)
            "#,
        )
        .bind(workspace_id)
        .bind(fixture.actor_id)
        .bind(fixture.principal_id)
        .bind(fixture.now)
        .execute(pool)
        .await
        .expect("workspace memberships should be seeded");
    }
    (workspace_a, workspace_b)
}

async fn assert_workspace_membership(
    pool: &sqlx::PgPool,
    workspace_id: uuid::Uuid,
    principal_id: uuid::Uuid,
    role: &str,
    status: &str,
) {
    let row = sqlx::query_as::<_, (String, String)>(
        r#"
        SELECT role::text, status::text
        FROM workspace_memberships
        WHERE workspace_id = $1 AND principal_id = $2
        "#,
    )
    .bind(workspace_id)
    .bind(principal_id)
    .fetch_one(pool)
    .await
    .expect("workspace membership should be readable");
    assert_eq!(row, (role.to_string(), status.to_string()));
}
