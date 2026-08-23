use tonic::Code;

use crate::{
    grpc::{invitations, pb::nvbes::enterprise::v1 as enterprise},
    test_support::{
        cleanup_tenant, has_enterprise_contract_schema, seed_enterprise_fixture, test_pool,
    },
};

#[tokio::test]
async fn invitations_round_trip_through_enterprise_store() {
    let pool = test_pool();
    if !has_enterprise_contract_schema(&pool).await {
        eprintln!("skipping test: local database is missing Enterprise contract schema");
        return;
    }

    let fixture = seed_enterprise_fixture(&pool, "invitations").await;
    let workspace_id = seed_invitation_workspace(&pool, &fixture).await;
    let response = invitations::create_invitations(
        &pool,
        fixture.tenant_id,
        fixture.actor_id,
        enterprise::CreateInvitationsRequest {
            context: None,
            tenant_id: fixture.tenant_id.to_string(),
            emails: vec!["  New.Member@Example.TEST ".to_string()],
            role: "member".to_string(),
            workspace_ids: vec![workspace_id.to_string()],
            scope: "tenant".to_string(),
            organization_id: String::new(),
        },
    )
    .await
    .expect("invitation should be created");

    assert_eq!(response.invitations.len(), 1);
    let invitation = &response.invitations[0];
    assert_eq!(invitation.email, "new.member@example.test");
    assert_eq!(invitation.role, "member");
    assert_eq!(invitation.workspace_ids, vec![workspace_id.to_string()]);
    assert_eq!(invitation.status, "pending");
    assert!(!invitation.invitation_id.is_empty());
    assert!(!invitation.invited_at.is_empty());
    assert!(!invitation.expires_at.is_empty());

    let audit_count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM audit_events
        WHERE tenant_id = $1
          AND action = 'enterprise.member.invited'
          AND target_id = $2
        "#,
    )
    .bind(fixture.tenant_id)
    .bind(uuid::Uuid::parse_str(&invitation.invitation_id).expect("invitation id should parse"))
    .fetch_one(&pool)
    .await
    .expect("audit should be readable");
    assert_eq!(audit_count, 1);

    let duplicate = invitations::create_invitations(
        &pool,
        fixture.tenant_id,
        fixture.actor_id,
        enterprise::CreateInvitationsRequest {
            context: None,
            tenant_id: fixture.tenant_id.to_string(),
            emails: vec!["new.member@example.test".to_string()],
            role: "member".to_string(),
            workspace_ids: vec![workspace_id.to_string()],
            scope: "tenant".to_string(),
            organization_id: String::new(),
        },
    )
    .await
    .expect_err("duplicate pending invitation should be rejected");
    assert_eq!(duplicate.code(), Code::AlreadyExists);

    cleanup_tenant(&pool, fixture.tenant_id).await;
}

async fn seed_invitation_workspace(
    pool: &sqlx::PgPool,
    fixture: &crate::test_support::EnterpriseFixture,
) -> uuid::Uuid {
    let workspace_id = uuid::Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO workspaces (
          id, tenant_id, name, workspace_type, owner_user_id, plan_code, created_at, updated_at
        )
        VALUES ($1, $2, 'Enterprise invitation workspace', 'team', $3, 'team', $4, $4)
        "#,
    )
    .bind(workspace_id)
    .bind(fixture.tenant_id)
    .bind(fixture.actor_id)
    .bind(fixture.now)
    .execute(pool)
    .await
    .expect("workspace should be seeded");
    workspace_id
}
