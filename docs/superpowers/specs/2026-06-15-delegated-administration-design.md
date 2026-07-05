# Design Spec: Delegated Administration for Organization Admins

Allow organization-scoped administrators (members of `organization_memberships` with roles `owner`, `admin`, or `security_admin`) to manage only their organization's workspaces, users, invitations, and audit logs.

## Architectural Changes

### 1. `AdminScope` and Resolution

Add a new `AdminScope` enum to represent the administration boundary:
- `AdminScope::Tenant`: Full tenant administrative access.
- `AdminScope::Organization(Uuid)`: Administrative access restricted to a specific organization.

```rust
// apps/account-service/src/identity.domains.authz.types.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdminScope {
    Tenant,
    Organization(Uuid),
}
```

Implement a scope resolution function in the authorization domain:
```rust
// apps/account-service/src/identity.domains.authz.service.rs
pub async fn resolve_admin_scope(
    db: &PgPool,
    auth: &impl TenantManagementAuth,
    tenant_id: Uuid,
    organization_id: Option<Uuid>,
) -> Result<AdminScope, AppError> {
    ensure_tenant_context(auth, tenant_id)?;

    // Check tenant membership
    let tenant_role = sqlx::query_scalar::<_, String>(
        r#"
        SELECT role::text
        FROM tenant_memberships
        WHERE tenant_id = $1
          AND principal_id = $2
          AND status = 'active'
        LIMIT 1
        "#,
    )
    .bind(tenant_id)
    .bind(auth.user_id())
    .fetch_optional(db)
    .await?;

    let is_tenant_admin = tenant_role
        .as_deref()
        .and_then(parse_identity_role)
        .is_some_and(identity_role_allows_tenant_management);

    if is_tenant_admin {
        return Ok(AdminScope::Tenant);
    }

    // Check organization membership
    if let Some(org_id) = organization_id {
        let belongs = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS (SELECT 1 FROM organizations WHERE id = $1 AND tenant_id = $2)",
        )
        .bind(org_id)
        .bind(tenant_id)
        .fetch_one(db)
        .await?;

        if belongs {
            let org_role = sqlx::query_scalar::<_, String>(
                r#"
                SELECT role::text
                FROM organization_memberships
                WHERE organization_id = $1
                  AND principal_id = $2
                  AND status = 'active'
                LIMIT 1
                "#,
            )
            .bind(org_id)
            .bind(auth.user_id())
            .fetch_optional(db)
            .await?;

            let is_org_admin = org_role
                .as_deref()
                .and_then(parse_identity_role)
                .is_some_and(identity_role_allows_tenant_management);

            if is_org_admin {
                return Ok(AdminScope::Organization(org_id));
            }
        }
    }

    Err(AppError::forbidden(
        "tenant_management_denied",
        "You do not have permission to manage this tenant or organization.",
    ))
}
```

### 2. Database Scoping

Update reads and writes queries to support scoping:

- **`list_users`**: Only lists users who are members of the given organization when scoped.
- **`list_invitations`**: Filters pending invitations to workspaces belonging to the organization.
- **`list_workspaces`**: Filters workspaces belonging to the organization.
- **`list_audit_events`**: Filters events to those targeting or occurring in workspaces belonging to the organization.
- **`usage_metrics`**: Scopes workspace metrics and storage metrics to the organization.
- **`ensure_workspaces_belong`**: Verifies workspace IDs belong to both the tenant and the scoped organization.

### 3. Mutations Security Boundaries

Implement `ensure_user_in_organization` check for:
- Updating user access.
- Suspending a user.
- Reactivating a user.

### 4. Tenant-Wide Endpoint Blocking

Global policies, billing details, trust centers, security alerts, and developer settings will return a `forbidden` error when accessed with `AdminScope::Organization`.
