use crate::domains::enterprise::service::{
    create_invitations, get_billing, get_security, list_audit_events, list_users, list_workspaces,
    update_user_access,
};
use crate::domains::enterprise::types::{
    EnterpriseAccessUpdateInput, EnterpriseInvitationInput, EnterpriseRole,
};
use crate::http::middleware::jwt::AuthContext;
use chrono::Utc;
use uuid::Uuid;

fn mock_auth_context(principal_id: Uuid, email: &str, tenant_id: Uuid, org_id: Option<Uuid>) -> AuthContext {
    AuthContext {
        user_id: principal_id,
        user_email: email.to_string(),
        display_name: "Test User".to_string(),
        email_verified_at: Some(Utc::now()),
        mfa_enabled: true,
        tenant_id: Some(tenant_id),
        organization_id: org_id,
        workspace_id: None,
        workspace_region: None,
        token_type: "access".to_string(),
        scope: "openid profile email".to_string(),
        jti: Uuid::new_v4().to_string(),
        session_id: Uuid::new_v4(),
        acr: Some("aal2".to_string()),
        amr: vec!["pwd".to_string(), "otp".to_string()],
        auth_time: Some(Utc::now().timestamp()),
        client_id: None,
        cnf_jkt: None,
    }
}

async fn setup_test_data(
    pool: &sqlx::PgPool,
) -> (
    Uuid,         // tenant_id
    Uuid,         // org_a_id
    Uuid,         // org_b_id
    Uuid,         // workspace_a_id
    Uuid,         // workspace_b_id
    Uuid,         // user_a_id
    Uuid,         // user_b_id
    AuthContext,  // org_a_admin_auth
) {
    let tenant_id = Uuid::new_v4();
    let org_a_id = Uuid::new_v4();
    let org_b_id = Uuid::new_v4();
    let workspace_a_id = Uuid::new_v4();
    let workspace_b_id = Uuid::new_v4();
    let admin_a_id = Uuid::new_v4();
    let user_a_id = Uuid::new_v4();
    let user_b_id = Uuid::new_v4();
    let now = Utc::now();

    // 1. Tenant
    sqlx::query(
        r#"
        INSERT INTO tenants (id, kind, name, slug, status, security_tier, created_at, updated_at)
        VALUES ($1, 'personal', 'Test Tenant', $2, 'active', 'standard', $3, $3)
        "#,
    )
    .bind(tenant_id)
    .bind(format!("tenant-{}", tenant_id))
    .bind(now)
    .execute(pool)
    .await
    .unwrap();

    // 2. Principals
    for pid in &[admin_a_id, user_a_id, user_b_id] {
        sqlx::query(
            "INSERT INTO principals (id, tenant_id, principal_kind, status, display_name, created_at, updated_at) VALUES ($1, $2, 'human', 'active', 'Test User', $3, $3)",
        )
        .bind(pid)
        .bind(tenant_id)
        .bind(now)
        .execute(pool)
        .await
        .unwrap();
    }

    // 3. Users
    sqlx::query(
        "INSERT INTO users (principal_id, email, username, firstname, lastname, created_at, updated_at) VALUES ($1, $2, $3, 'Admin', 'A', $4, $4)",
    )
    .bind(admin_a_id)
    .bind(format!("admin_a_{}@example.test", admin_a_id))
    .bind(format!("admin_a_{}", admin_a_id))
    .bind(now)
    .execute(pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO users (principal_id, email, username, firstname, lastname, created_at, updated_at) VALUES ($1, $2, $3, 'User', 'A', $4, $4)",
    )
    .bind(user_a_id)
    .bind(format!("user_a_{}@example.test", user_a_id))
    .bind(format!("user_a_{}", user_a_id))
    .bind(now)
    .execute(pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO users (principal_id, email, username, firstname, lastname, created_at, updated_at) VALUES ($1, $2, $3, 'User', 'B', $4, $4)",
    )
    .bind(user_b_id)
    .bind(format!("user_b_{}@example.test", user_b_id))
    .bind(format!("user_b_{}", user_b_id))
    .bind(now)
    .execute(pool)
    .await
    .unwrap();

    // 4. Tenant memberships
    for pid in &[admin_a_id, user_a_id, user_b_id] {
        sqlx::query(
            "INSERT INTO tenant_memberships (tenant_id, principal_id, principal_kind, role, status, created_at, updated_at) VALUES ($1, $2, 'human', 'member', 'active', $3, $3)",
        )
        .bind(tenant_id)
        .bind(pid)
        .bind(now)
        .execute(pool)
        .await
        .unwrap();
    }

    // 5. Organizations
    sqlx::query(
        "INSERT INTO organizations (id, tenant_id, name, slug, status, created_at, updated_at) VALUES ($1, $2, 'Org A', $3, 'active', $4, $4)",
    )
    .bind(org_a_id)
    .bind(tenant_id)
    .bind(format!("org-a-{}", org_a_id))
    .bind(now)
    .execute(pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO organizations (id, tenant_id, name, slug, status, created_at, updated_at) VALUES ($1, $2, 'Org B', $3, 'active', $4, $4)",
    )
    .bind(org_b_id)
    .bind(tenant_id)
    .bind(format!("org-b-{}", org_b_id))
    .bind(now)
    .execute(pool)
    .await
    .unwrap();

    // 6. Organization memberships
    sqlx::query(
        "INSERT INTO organization_memberships (organization_id, principal_id, role, status, created_at, updated_at) VALUES ($1, $2, 'admin', 'active', $3, $3)",
    )
    .bind(org_a_id)
    .bind(admin_a_id)
    .bind(now)
    .execute(pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO organization_memberships (organization_id, principal_id, role, status, created_at, updated_at) VALUES ($1, $2, 'member', 'active', $3, $3)",
    )
    .bind(org_a_id)
    .bind(user_a_id)
    .bind(now)
    .execute(pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO organization_memberships (organization_id, principal_id, role, status, created_at, updated_at) VALUES ($1, $2, 'member', 'active', $3, $3)",
    )
    .bind(org_b_id)
    .bind(user_b_id)
    .bind(now)
    .execute(pool)
    .await
    .unwrap();

    // 7. Workspaces
    sqlx::query(
        "INSERT INTO workspaces (id, tenant_id, organization_id, name, workspace_type, plan_code, created_at, updated_at) VALUES ($1, $2, $3, 'Workspace A', 'team', 'team', $4, $4)",
    )
    .bind(workspace_a_id)
    .bind(tenant_id)
    .bind(org_a_id)
    .bind(now)
    .execute(pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO workspaces (id, tenant_id, organization_id, name, workspace_type, plan_code, created_at, updated_at) VALUES ($1, $2, $3, 'Workspace B', 'team', 'team', $4, $4)",
    )
    .bind(workspace_b_id)
    .bind(tenant_id)
    .bind(org_b_id)
    .bind(now)
    .execute(pool)
    .await
    .unwrap();

    // 8. Workspace memberships
    sqlx::query(
        "INSERT INTO workspace_memberships (workspace_id, principal_id, role, status, created_at, updated_at) VALUES ($1, $2, 'member', 'active', $3, $3)",
    )
    .bind(workspace_a_id)
    .bind(user_a_id)
    .bind(now)
    .execute(pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO workspace_memberships (workspace_id, principal_id, role, status, created_at, updated_at) VALUES ($1, $2, 'member', 'active', $3, $3)",
    )
    .bind(workspace_b_id)
    .bind(user_b_id)
    .bind(now)
    .execute(pool)
    .await
    .unwrap();

    let org_a_admin_auth = mock_auth_context(
        admin_a_id,
        &format!("admin_a_{}@example.test", admin_a_id),
        tenant_id,
        Some(org_a_id),
    );

    (
        tenant_id,
        org_a_id,
        org_b_id,
        workspace_a_id,
        workspace_b_id,
        user_a_id,
        user_b_id,
        org_a_admin_auth,
    )
}

#[tokio::test]
async fn test_delegated_administration_scoping_and_blocking() {
    let pool = crate::test_support::shared_test_pool();
    crate::test_support::ensure_test_database(&pool).await;
    let redis = crate::test_support::test_redis_pool().await;

    let (
        tenant_id,
        _org_a_id,
        _org_b_id,
        workspace_a_id,
        workspace_b_id,
        user_a_id,
        user_b_id,
        org_a_admin_auth,
    ) = setup_test_data(&pool).await;

    // --- 1. Test Scoped Reads (Users, Workspaces, Audit Events) ---

    // A. List Users
    let users_resp = list_users(&pool, &org_a_admin_auth, tenant_id).await.unwrap();
    // Org A admin should see: admin_a (self) and user_a, but NOT user_b (which is in Org B)
    assert!(users_resp.users.iter().any(|u| u.id == org_a_admin_auth.user_id));
    assert!(users_resp.users.iter().any(|u| u.id == user_a_id));
    assert!(!users_resp.users.iter().any(|u| u.id == user_b_id));

    // B. List Workspaces
    let workspaces_resp = list_workspaces(&pool, &org_a_admin_auth, tenant_id).await.unwrap();
    // Org A admin should see: workspace_a, but NOT workspace_b
    assert!(workspaces_resp.workspaces.iter().any(|w| w.id == workspace_a_id));
    assert!(!workspaces_resp.workspaces.iter().any(|w| w.id == workspace_b_id));

    // C. List Audit Events
    let audit_resp = list_audit_events(&pool, &org_a_admin_auth, tenant_id).await.unwrap();
    // Initially empty or scoped only to workspace_a
    for event in audit_resp.events {
        if let Some(ws_id) = event
            .metadata
            .as_ref()
            .and_then(|m| m.get("workspace_id"))
            .and_then(|v| v.as_str())
        {
            assert_eq!(ws_id, workspace_a_id.to_string());
        }
    }

    // --- 2. Test Blocked Tenant-wide Reads ---

    // A. Billing should fail
    let billing_err = get_billing(&pool, &org_a_admin_auth, tenant_id).await.unwrap_err();
    assert_eq!(billing_err.code, "tenant_scope_required");

    // B. Security center should fail
    let security_err = get_security(&pool, &org_a_admin_auth, tenant_id, 12).await.unwrap_err();
    assert_eq!(security_err.code, "tenant_scope_required");

    // --- 3. Test Scoped Mutations ---

    // A. Create Invitation
    // Inviting to workspace_a (in Org A) should succeed
    let invite_ok = create_invitations(
        &pool,
        &redis,
        &org_a_admin_auth,
        tenant_id,
        EnterpriseInvitationInput {
            emails: vec!["new_member@example.test".to_string()],
            role: EnterpriseRole::Member,
            module_grants: vec![],
            workspace_ids: vec![workspace_a_id],
        },
    )
    .await;
    assert!(invite_ok.is_ok());

    // Inviting to workspace_b (in Org B) should fail
    let invite_err = create_invitations(
        &pool,
        &redis,
        &org_a_admin_auth,
        tenant_id,
        EnterpriseInvitationInput {
            emails: vec!["new_member_blocked@example.test".to_string()],
            role: EnterpriseRole::Member,
            module_grants: vec![],
            workspace_ids: vec![workspace_b_id],
        },
    )
    .await
    .unwrap_err();
    assert_eq!(invite_err.code, "invalid_workspace_scope");

    // B. Update User Access
    // Updating user_a (in Org A) should succeed
    let update_ok = update_user_access(
        &pool,
        &redis,
        &org_a_admin_auth,
        tenant_id,
        user_a_id,
        EnterpriseAccessUpdateInput {
            role: EnterpriseRole::Member,
            module_grants: vec![],
            workspace_ids: vec![workspace_a_id],
        },
    )
    .await;
    assert!(update_ok.is_ok());

    // Updating user_b (in Org B) should fail
    let update_err = update_user_access(
        &pool,
        &redis,
        &org_a_admin_auth,
        tenant_id,
        user_b_id,
        EnterpriseAccessUpdateInput {
            role: EnterpriseRole::Member,
            module_grants: vec![],
            workspace_ids: vec![workspace_a_id],
        },
    )
    .await
    .unwrap_err();
    assert_eq!(update_err.code, "user_not_in_organization");
}
