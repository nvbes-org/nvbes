use super::{identity_role_allows_tenant_management, resolve_admin_scope};
use crate::domains::authz::{AdminScope, TenantManagementAuth};
use nvbes_core::authz::IdentityRole;

#[test]
fn tenant_management_is_limited_to_identity_admin_roles() {
    assert!(identity_role_allows_tenant_management(IdentityRole::Owner));
    assert!(identity_role_allows_tenant_management(IdentityRole::Admin));
    assert!(identity_role_allows_tenant_management(
        IdentityRole::SecurityAdmin
    ));
    assert!(!identity_role_allows_tenant_management(
        IdentityRole::BillingAdmin
    ));
    assert!(!identity_role_allows_tenant_management(
        IdentityRole::Member
    ));
}

#[tokio::test]
async fn test_resolve_admin_scope() {
    let pool = crate::test_support::shared_test_pool();
    if !crate::test_support::is_test_database_available(&pool).await {
        eprintln!("skipping test: database not available");
        return;
    }
    crate::test_support::ensure_test_database(&pool).await;

    let tenant_id = uuid::Uuid::new_v4();
    let principal_id = uuid::Uuid::new_v4();
    let org_id = uuid::Uuid::new_v4();
    let now = chrono::Utc::now();

    sqlx::query(
        r#"
        INSERT INTO tenants (id, kind, name, slug, status, security_tier, created_at, updated_at)
        VALUES ($1, 'personal', 'Test Tenant', $2, 'active', 'standard', $3, $3)
        "#,
    )
    .bind(tenant_id)
    .bind(format!("tenant-{}", tenant_id))
    .bind(now)
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO principals (id, tenant_id, principal_kind, status, display_name, created_at, updated_at) VALUES ($1, $2, 'human', 'active', 'Test User', $3, $3)",
    )
    .bind(principal_id)
    .bind(tenant_id)
    .bind(now)
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO organizations (id, tenant_id, name, slug, status, created_at, updated_at) VALUES ($1, $2, 'Test Org', $3, 'active', $4, $4)",
    )
    .bind(org_id)
    .bind(tenant_id)
    .bind(format!("org-{}", org_id))
    .bind(now)
    .execute(&pool)
    .await
    .unwrap();

    struct MockAuth {
        user_id: uuid::Uuid,
        tenant_id: Option<uuid::Uuid>,
        email_verified_at: Option<chrono::DateTime<chrono::Utc>>,
    }

    impl TenantManagementAuth for MockAuth {
        fn user_id(&self) -> uuid::Uuid {
            self.user_id
        }

        fn tenant_id(&self) -> Option<uuid::Uuid> {
            self.tenant_id
        }

        fn email_verified_at(&self) -> Option<chrono::DateTime<chrono::Utc>> {
            self.email_verified_at
        }
    }

    let auth = MockAuth {
        user_id: principal_id,
        tenant_id: Some(tenant_id),
        email_verified_at: Some(now),
    };

    let result = resolve_admin_scope(&pool, &auth, tenant_id, Some(org_id)).await;
    assert!(result.is_err());

    sqlx::query(
        "INSERT INTO tenant_memberships (tenant_id, principal_id, principal_kind, role, status, created_at, updated_at) VALUES ($1, $2, 'human', 'admin', 'active', $3, $3)",
    )
    .bind(tenant_id)
    .bind(principal_id)
    .bind(now)
    .execute(&pool)
    .await
    .unwrap();

    let scope = resolve_admin_scope(&pool, &auth, tenant_id, Some(org_id))
        .await
        .unwrap();
    assert_eq!(scope, AdminScope::Tenant);

    sqlx::query("DELETE FROM tenant_memberships WHERE tenant_id = $1 AND principal_id = $2")
        .bind(tenant_id)
        .bind(principal_id)
        .execute(&pool)
        .await
        .unwrap();

    sqlx::query(
        "INSERT INTO organization_memberships (organization_id, principal_id, role, status, created_at, updated_at) VALUES ($1, $2, 'admin', 'active', $3, $3)",
    )
    .bind(org_id)
    .bind(principal_id)
    .bind(now)
    .execute(&pool)
    .await
    .unwrap();

    let scope = resolve_admin_scope(&pool, &auth, tenant_id, Some(org_id))
        .await
        .unwrap();
    assert_eq!(scope, AdminScope::Organization(org_id));
}
