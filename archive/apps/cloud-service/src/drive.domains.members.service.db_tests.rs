use chrono::{Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::{list_invitations, list_members};
use crate::{
    domains::{authz::WorkspaceRole, members::types::ListMembersInput},
    test_support::{seed_workspace, test_pool, workspace_access},
};

async fn supports_current_member_schema(pool: &PgPool) -> bool {
    sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
          SELECT 1 FROM information_schema.columns
          WHERE table_name = 'users' AND column_name = 'display_name'
        ) AND EXISTS (
          SELECT 1 FROM information_schema.tables
          WHERE table_name = 'workspace_memberships'
        )
        "#,
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false)
}

async fn seed_member(pool: &PgPool, workspace_id: Uuid, email: &str) {
    let user_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO users (id, email, email_verified_at, display_name, status, mfa_enabled)
        VALUES ($1, $2, NOW(), $2, 'active', false)
        "#,
    )
    .bind(user_id)
    .bind(email)
    .execute(pool)
    .await
    .expect("member user insert should succeed");
    sqlx::query(
        r#"
        INSERT INTO workspace_memberships (workspace_id, user_id, role, status, source)
        VALUES ($1, $2, 'admin', 'active', 'test')
        "#,
    )
    .bind(workspace_id)
    .bind(user_id)
    .execute(pool)
    .await
    .expect("workspace membership insert should succeed");
}

async fn seed_invitation(
    pool: &PgPool,
    workspace_id: Uuid,
    email: &str,
    created_at: chrono::DateTime<Utc>,
) {
    let invitation_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO workspace_invitations (
          id, workspace_id, email, role, status, invited_by, token_hash, expires_at,
          created_at, updated_at
        )
        VALUES ($1, $2, $3, 'member', 'pending', $4, $5, $6, $7, $7)
        "#,
    )
    .bind(invitation_id)
    .bind(workspace_id)
    .bind(email)
    .bind(Uuid::new_v4())
    .bind(format!("token-{invitation_id}"))
    .bind(created_at + Duration::days(7))
    .bind(created_at)
    .execute(pool)
    .await
    .expect("workspace invitation insert should succeed");
}

#[tokio::test]
async fn member_pages_are_stable_and_filtered() {
    let pool = test_pool();
    if !supports_current_member_schema(&pool).await {
        eprintln!("skipping test: local database is missing the current Cloud member schema");
        return;
    }
    let key = format!("member-pagination-{}", Uuid::new_v4());
    let (principal_id, workspace_id, tenant_id) = seed_workspace(&pool, &key).await;
    seed_member(&pool, workspace_id, "team_1@example.com").await;
    seed_member(&pool, workspace_id, "team_2@example.com").await;
    seed_member(&pool, workspace_id, "teamx@example.com").await;
    let access = workspace_access(principal_id, workspace_id, tenant_id);

    let first = list_members(
        &pool,
        &access,
        ListMembersInput {
            limit: Some(1),
            cursor: None,
            role: Some(WorkspaceRole::Admin),
            email_prefix: Some("team_".to_string()),
        },
    )
    .await
    .expect("first member page should load");
    assert_eq!(first.members.len(), 1);
    assert!(first.members_has_more);

    let second = list_members(
        &pool,
        &access,
        ListMembersInput {
            limit: Some(1),
            cursor: first.members_next_cursor,
            role: Some(WorkspaceRole::Admin),
            email_prefix: Some("team_".to_string()),
        },
    )
    .await
    .expect("second member page should load");
    assert_eq!(second.members.len(), 1);
    assert!(!second.members_has_more);
    assert!(second.members_next_cursor.is_none());
}

#[tokio::test]
async fn invitation_pages_are_stable_and_filtered() {
    let pool = test_pool();
    if !supports_current_member_schema(&pool).await {
        eprintln!("skipping test: local database is missing the current Cloud member schema");
        return;
    }
    let key = format!("invitation-pagination-{}", Uuid::new_v4());
    let (principal_id, workspace_id, tenant_id) = seed_workspace(&pool, &key).await;
    let access = workspace_access(principal_id, workspace_id, tenant_id);
    let now = Utc::now();
    seed_invitation(
        &pool,
        workspace_id,
        "alpha@example.com",
        now - Duration::minutes(2),
    )
    .await;
    seed_invitation(
        &pool,
        workspace_id,
        "bravo@example.com",
        now - Duration::minutes(1),
    )
    .await;
    seed_invitation(&pool, workspace_id, "charlie@example.com", now).await;

    let first = list_invitations(&pool, &access, Some(1), None, Vec::new())
        .await
        .expect("first invitation page should load");
    assert_eq!(first.invitations.len(), 1);
    assert!(first.has_more);

    let second = list_invitations(
        &pool,
        &access,
        Some(1),
        first.next_cursor,
        vec!["pending".to_string()],
    )
    .await
    .expect("second invitation page should load");
    assert_eq!(second.invitations.len(), 1);
    assert!(second.has_more);
    assert!(second.next_cursor.is_some());
}
