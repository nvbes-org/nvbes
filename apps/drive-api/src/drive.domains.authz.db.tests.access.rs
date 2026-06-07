use chrono::Utc;
use uuid::Uuid;

use super::support::*;
use crate::domains::auth::types::{AuthContext, AuthPrincipalKind};
use crate::domains::authz::types::WorkspaceRole;

#[tokio::test]
async fn load_workspace_access_accepts_workspace_scoped_service_account() {
    let pool = test_pool();
    if !db_supports_current_drive_schema(&pool).await {
        eprintln!("skipping test: local database is missing current drive schema columns");
        return;
    }
    let key = format!("drive-authz-{}", Uuid::new_v4());
    let (_owner_user_id, workspace_id, tenant_id) = seed_workspace(&pool, &key).await;
    let principal_id = Uuid::new_v4();

    let access = crate::domains::authz::db::load_workspace_access(
        &pool,
        &machine_auth(principal_id, tenant_id, workspace_id, "member"),
        workspace_id,
    )
    .await
    .expect("service account should access its workspace");

    assert_eq!(access.workspace_id, workspace_id);
    assert_eq!(access.tenant_id, Some(tenant_id));
    assert_eq!(access.role, WorkspaceRole::Member);

    cleanup(&pool, &key).await;
}

#[tokio::test]
async fn load_workspace_access_rejects_workspace_mismatch_for_service_account() {
    let pool = test_pool();
    if !db_supports_current_drive_schema(&pool).await {
        eprintln!("skipping test: local database is missing current drive schema columns");
        return;
    }
    let key = format!("drive-authz-mismatch-{}", Uuid::new_v4());
    let (_owner_user_id, workspace_id, tenant_id) = seed_workspace(&pool, &key).await;
    let principal_id = Uuid::new_v4();

    let error = crate::domains::authz::db::load_workspace_access(
        &pool,
        &machine_auth(principal_id, tenant_id, Uuid::new_v4(), "member"),
        workspace_id,
    )
    .await
    .expect_err("service account should be rejected on workspace mismatch");

    assert_eq!(error.code, "workspace_context_mismatch");

    cleanup(&pool, &key).await;
}

#[tokio::test]
async fn load_workspace_access_reduces_effective_role_for_delegated_actor() {
    let pool = test_pool();
    if !db_supports_current_drive_schema(&pool).await {
        eprintln!("skipping test: local database is missing current drive schema columns");
        return;
    }
    let key = format!("drive-authz-delegated-{}", Uuid::new_v4());
    let (user_id, workspace_id, tenant_id) = seed_workspace(&pool, &key).await;

    sqlx::query(
        r#"
        INSERT INTO workspace_memberships (workspace_id, user_id, role, status, source, created_at, updated_at)
        VALUES ($1, $2, 'owner', 'active', 'manual', $3, $3)
        ON CONFLICT (workspace_id, user_id) DO NOTHING
        "#,
    )
    .bind(workspace_id)
    .bind(user_id)
    .bind(Utc::now())
    .execute(&pool)
    .await
    .expect("owner membership insert should succeed");

    let access = crate::domains::authz::db::load_workspace_access(
        &pool,
        &delegated_user_auth(user_id, tenant_id, workspace_id, "viewer"),
        workspace_id,
    )
    .await
    .expect("delegated user should load workspace access");

    assert_eq!(access.role, WorkspaceRole::Viewer);

    cleanup(&pool, &key).await;
}

#[tokio::test]
async fn load_workspace_access_accepts_user_role_from_token() {
    let pool = test_pool();
    if !db_supports_current_drive_schema(&pool).await {
        eprintln!("skipping test: local database is missing current drive schema columns");
        return;
    }
    let key = format!("drive-authz-user-token-{}", Uuid::new_v4());
    let (user_id, workspace_id, tenant_id) = seed_workspace(&pool, &key).await;

    let auth = AuthContext {
        principal_id: user_id,
        principal_kind: AuthPrincipalKind::User,
        user_id,
        email_verified_at: Some(Utc::now()),
        session_id: Uuid::new_v4(),
        tenant_id: Some(tenant_id),
        organization_id: None,
        workspace_id: Some(workspace_id),
        scope: "drive.files.read drive.workspace.read".to_string(),
        role: Some("admin".to_string()),
        amr: vec!["pwd".to_string()],
        actor: None,
        acr: Some("aal1".to_string()),
        auth_time: Some(Utc::now()),
    };

    let access = crate::domains::authz::db::load_workspace_access(&pool, &auth, workspace_id)
        .await
        .expect("user with token role should load workspace access");

    assert_eq!(access.workspace_id, workspace_id);
    assert_eq!(access.role, WorkspaceRole::Admin);

    cleanup(&pool, &key).await;
}
