use crate::http::middleware::jwt::AuthContext;
use chrono::Utc;
use uuid::Uuid;

fn mock_auth_context(
    principal_id: Uuid,
    email: &str,
    tenant_id: Uuid,
    org_id: Option<Uuid>,
) -> AuthContext {
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

pub(super) async fn setup_delegated_admin_test_data(
    pool: &sqlx::PgPool,
) -> (
    Uuid,        // tenant_id
    Uuid,        // org_a_id
    Uuid,        // org_b_id
    Uuid,        // workspace_a_id
    Uuid,        // workspace_b_id
    Uuid,        // user_a_id
    Uuid,        // user_b_id
    AuthContext, // org_a_admin_auth
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

    insert_user(pool, admin_a_id, "admin_a", "Admin", "A", now).await;
    insert_user(pool, user_a_id, "user_a", "User", "A", now).await;
    insert_user(pool, user_b_id, "user_b", "User", "B", now).await;

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

    insert_org(pool, tenant_id, org_a_id, "Org A", "org-a", now).await;
    insert_org(pool, tenant_id, org_b_id, "Org B", "org-b", now).await;

    insert_org_membership(pool, org_a_id, admin_a_id, "admin", now).await;
    insert_org_membership(pool, org_a_id, user_a_id, "member", now).await;
    insert_org_membership(pool, org_b_id, user_b_id, "member", now).await;

    insert_workspace(
        pool,
        tenant_id,
        org_a_id,
        workspace_a_id,
        "Workspace A",
        now,
    )
    .await;
    insert_workspace(
        pool,
        tenant_id,
        org_b_id,
        workspace_b_id,
        "Workspace B",
        now,
    )
    .await;

    insert_workspace_membership(pool, workspace_a_id, user_a_id, now).await;
    insert_workspace_membership(pool, workspace_b_id, user_b_id, now).await;
    insert_workspace_membership(pool, workspace_b_id, user_a_id, now).await;

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

async fn insert_user(
    pool: &sqlx::PgPool,
    principal_id: Uuid,
    username_prefix: &str,
    firstname: &str,
    lastname: &str,
    now: chrono::DateTime<Utc>,
) {
    sqlx::query(
        "INSERT INTO users (principal_id, email, username, firstname, lastname, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $6)",
    )
    .bind(principal_id)
    .bind(format!("{username_prefix}_{principal_id}@example.test"))
    .bind(format!("{username_prefix}_{principal_id}"))
    .bind(firstname)
    .bind(lastname)
    .bind(now)
    .execute(pool)
    .await
    .unwrap();
}

async fn insert_org(
    pool: &sqlx::PgPool,
    tenant_id: Uuid,
    org_id: Uuid,
    name: &str,
    slug_prefix: &str,
    now: chrono::DateTime<Utc>,
) {
    sqlx::query(
        "INSERT INTO organizations (id, tenant_id, name, slug, status, created_at, updated_at) VALUES ($1, $2, $3, $4, 'active', $5, $5)",
    )
    .bind(org_id)
    .bind(tenant_id)
    .bind(name)
    .bind(format!("{slug_prefix}-{org_id}"))
    .bind(now)
    .execute(pool)
    .await
    .unwrap();
}

async fn insert_org_membership(
    pool: &sqlx::PgPool,
    org_id: Uuid,
    principal_id: Uuid,
    role: &str,
    now: chrono::DateTime<Utc>,
) {
    sqlx::query(
        "INSERT INTO organization_memberships (organization_id, principal_id, role, status, created_at, updated_at) VALUES ($1, $2, $3::identity_role, 'active', $4, $4)",
    )
    .bind(org_id)
    .bind(principal_id)
    .bind(role)
    .bind(now)
    .execute(pool)
    .await
    .unwrap();
}

async fn insert_workspace(
    pool: &sqlx::PgPool,
    tenant_id: Uuid,
    org_id: Uuid,
    workspace_id: Uuid,
    name: &str,
    now: chrono::DateTime<Utc>,
) {
    sqlx::query(
        "INSERT INTO workspaces (id, tenant_id, organization_id, name, workspace_type, plan_code, created_at, updated_at) VALUES ($1, $2, $3, $4, 'team', 'team', $5, $5)",
    )
    .bind(workspace_id)
    .bind(tenant_id)
    .bind(org_id)
    .bind(name)
    .bind(now)
    .execute(pool)
    .await
    .unwrap();
}

async fn insert_workspace_membership(
    pool: &sqlx::PgPool,
    workspace_id: Uuid,
    principal_id: Uuid,
    now: chrono::DateTime<Utc>,
) {
    sqlx::query(
        "INSERT INTO workspace_memberships (workspace_id, principal_id, role, status, created_at, updated_at) VALUES ($1, $2, 'member', 'active', $3, $3)",
    )
    .bind(workspace_id)
    .bind(principal_id)
    .bind(now)
    .execute(pool)
    .await
    .unwrap();
}
