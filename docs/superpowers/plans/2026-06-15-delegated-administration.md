# Delegated Administration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Allow organization-scoped administrators to manage only their organization's workspaces, users, invitations, and audit logs while blocking tenant-wide views.

**Architecture:** Introduce a type-safe `AdminScope` resolved from the user's active tenant and organization context. Propagate this scope down through the service layer to database queries to filter workspace-bound resources, and block tenant-level actions when the scope is restricted to an organization.

**Tech Stack:** Rust (Axum, SQLx, PostgreSQL)

---

### Task 1: Introduce `AdminScope` and export in `authz` domain

**Files:**
- Create/Modify: `apps/account-service/src/identity.domains.authz.types.rs:8-12`
- Modify: `apps/account-service/src/identity.domains.authz.mod.rs:15-18`

- [ ] **Step 1: Define `AdminScope` enum**
Add the `AdminScope` enum definition to `apps/account-service/src/identity.domains.authz.types.rs`:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdminScope {
    Tenant,
    Organization(uuid::Uuid),
}
```

- [ ] **Step 2: Export `AdminScope` in `authz` module**
Modify `apps/account-service/src/identity.domains.authz.mod.rs` to export `AdminScope`:
```rust
pub use types::{
    AdminScope, ResourceContext, TenantManagementAuth, WorkspaceAccess, WorkspaceAction,
    WorkspaceDecision, WorkspacePolicy, WorkspaceRole, parse_action, parse_identity_role,
    parse_role,
};
```

- [ ] **Step 3: Compile check**
Run: `rtk cargo check -p nvbes-account-service`
Expected: PASS

- [ ] **Step 4: Commit**
```bash
git add apps/account-service/src/identity.domains.authz.types.rs apps/account-service/src/identity.domains.authz.mod.rs
git commit -m "feat(authz): add AdminScope enum and exports"
```

---

### Task 2: Implement `resolve_admin_scope` in `authz` service

**Files:**
- Modify: `apps/account-service/src/identity.domains.authz.service.rs`
- Modify: `apps/account-service/src/identity.domains.authz.mod.rs`

- [ ] **Step 1: Write `resolve_admin_scope` function**
Add `resolve_admin_scope` to `apps/account-service/src/identity.domains.authz.service.rs`:
```rust
pub async fn resolve_admin_scope(
    db: &PgPool,
    auth: &impl TenantManagementAuth,
    tenant_id: Uuid,
    organization_id: Option<Uuid>,
) -> Result<AdminScope, AppError> {
    ensure_tenant_context(auth, tenant_id)?;

    // 1. Check if tenant-level membership allows admin elevation
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

    // 2. Check organization-level membership role
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

- [ ] **Step 2: Export `resolve_admin_scope` in `authz` mod**
Modify `apps/account-service/src/identity.domains.authz.mod.rs` to export `resolve_admin_scope`:
```rust
pub use service::{
    authorize_workspace_action, decide_workspace_action, ensure_email_verified,
    ensure_tenant_management_access, resolve_admin_scope,
};
```

- [ ] **Step 3: Add unit tests for `resolve_admin_scope`**
Add unit test cases to `mod tests` inside `apps/account-service/src/identity.domains.authz.service.rs` to verify:
1. Tenant admin resolves to `AdminScope::Tenant`.
2. Org admin resolves to `AdminScope::Organization(org_id)`.
3. Non-admin / wrong tenant returns forbidden error.

- [ ] **Step 4: Run tests**
Run: `rtk cargo test -p nvbes-account-service --lib domains::authz`
Expected: PASS

- [ ] **Step 5: Commit**
```bash
git add apps/account-service/src/identity.domains.authz.service.rs apps/account-service/src/identity.domains.authz.mod.rs
git commit -m "feat(authz): implement resolve_admin_scope with unit tests"
```

---

### Task 3: Support `AdminScope` in Enterprise Actor Access Checks

**Files:**
- Modify: `apps/account-service/src/identity.domains.enterprise.service.access.rs`

- [ ] **Step 1: Update `require_actor_access` to support `AdminScope`**
Change `require_actor_access` in `apps/account-service/src/identity.domains.enterprise.service.access.rs` to accept `scope: AdminScope` and handle organization roles:
```rust
pub(super) async fn require_actor_access(
    db: &Database,
    auth: &AuthContext,
    tenant_id: Uuid,
    scope: AdminScope,
) -> Result<ActorAccess, AppError> {
    match scope {
        AdminScope::Tenant => {
            let row = db::actor_access(db, tenant_id, auth.user_id)
                .await?
                .ok_or_else(|| {
                    AppError::forbidden(
                        "tenant_management_denied",
                        "You do not have permission to manage this tenant.",
                    )
                })?;
            let role = policy::role_from_db(&row.role);
            let break_glass = match (
                row.break_glass_procedure_reference,
                row.break_glass_reason,
                row.break_glass_created_at,
            ) {
                (Some(procedure_reference), Some(reason), Some(created_at)) => {
                    Some(EnterpriseBreakGlassAccount {
                        procedure_reference,
                        reason,
                        created_at,
                        last_used_at: row.break_glass_last_used_at,
                    })
                }
                _ => None,
            };
            Ok(ActorAccess {
                grants: policy::grants_for_role(&role),
                role,
                break_glass: row.break_glass,
                break_glass_account: break_glass,
            })
        }
        AdminScope::Organization(org_id) => {
            let role_str = sqlx::query_scalar::<_, String>(
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
            .bind(auth.user_id)
            .fetch_optional(db)
            .await?
            .ok_or_else(|| {
                AppError::forbidden(
                    "organization_management_denied",
                    "You do not have permission to manage this organization.",
                )
            })?;

            let parsed_role = parse_identity_role(&role_str)?;
            let ent_role = match parsed_role {
                nvbes_core::authz::IdentityRole::Owner => EnterpriseRole::Owner,
                nvbes_core::authz::IdentityRole::Admin => EnterpriseRole::Admin,
                nvbes_core::authz::IdentityRole::SecurityAdmin => EnterpriseRole::Admin,
                _ => EnterpriseRole::Member,
            };

            Ok(ActorAccess {
                grants: policy::grants_for_role(&ent_role),
                role: ent_role,
                break_glass: false,
                break_glass_account: None,
            })
        }
    }
}
```

- [ ] **Step 2: Update `ensure_member_manager` to support `AdminScope`**
Change `ensure_member_manager` in `apps/account-service/src/identity.domains.enterprise.service.access.rs` to accept `scope: AdminScope` and skip active elevation requirement for organization scopes:
```rust
pub(super) async fn ensure_member_manager(
    db: &Database,
    redis: &nvbes_redis::RedisPool,
    auth: &AuthContext,
    tenant_id: Uuid,
    scope: AdminScope,
) -> Result<ActorAccess, AppError> {
    let access = require_actor_access(db, auth, tenant_id, scope).await?;
    if !policy::can_manage_members(
        policy::role_as_db(&access.role),
        &policy::grant_names(&access.grants),
    ) {
        return Err(AppError::forbidden(
            "members_grant_required",
            "Members access is required.",
        ));
    }
    if matches!(scope, AdminScope::Tenant) {
        crate::domains::enterprise::admin_elevation::require_active_admin_elevation(
            redis, auth, tenant_id,
        )
        .await?;
    }
    Ok(access)
}
```

- [ ] **Step 3: Update `fetch_user_view` to accept `scope: AdminScope`**
```rust
pub(super) async fn fetch_user_view(
    db: &Database,
    tenant_id: Uuid,
    user_id: Uuid,
    scope: AdminScope,
) -> Result<EnterpriseUser, AppError> {
    db::list_users(db, tenant_id, scope)
        .await?
        .into_iter()
        .find(|row| row.id == user_id)
        .map(db::EnterpriseUserRow::into_view)
        .ok_or_else(|| AppError::not_found("enterprise_user_not_found", "Tenant member not found."))
}
```

- [ ] **Step 4: Compile check**
Run: `rtk cargo check -p nvbes-account-service`
Expected: PASS (with some errors on callers we will fix in later tasks)

- [ ] **Step 5: Commit**
```bash
git add apps/account-service/src/identity.domains.enterprise.service.access.rs
git commit -m "feat(enterprise): update require_actor_access and ensure_member_manager for AdminScope"
```

---

### Task 4: Add Organization Scoping to Database Reads

**Files:**
- Modify: `apps/account-service/src/identity.domains.enterprise.db.reads.rs`
- Modify: `apps/account-service/src/identity.domains.enterprise.db.rs`

- [ ] **Step 1: Update `list_users` signature and implementation**
Modify `list_users` in `apps/account-service/src/identity.domains.enterprise.db.reads.rs` to take `scope: AdminScope` and filter by organization memberships when organization-scoped:
```rust
pub async fn list_users(
    db: &PgPool,
    tenant_id: Uuid,
    scope: AdminScope,
) -> Result<Vec<EnterpriseUserRow>, AppError> {
    match scope {
        AdminScope::Tenant => {
            Ok(sqlx::query_as::<_, EnterpriseUserRow>(r#"
                SELECT
                  u.principal_id AS id,
                  u.email,
                  COALESCE(NULLIF(concat_ws(' ', u.firstname, u.lastname), ''), u.username, u.email) AS display_name,
                  CASE MIN(CASE wm.role WHEN 'owner' THEN 0 WHEN 'admin' THEN 1 WHEN 'member' THEN 2 WHEN 'viewer' THEN 3 ELSE 4 END)
                    WHEN 0 THEN 'owner'
                    WHEN 1 THEN 'admin'
                    WHEN 2 THEN 'member'
                    ELSE 'viewer'
                  END AS role,
                  tbga.procedure_reference AS break_glass_procedure_reference,
                  tbga.reason AS break_glass_reason,
                  tbga.created_at AS break_glass_created_at,
                  tbga.last_used_at AS break_glass_last_used_at,
                  COALESCE(array_agg(DISTINCT w.id) FILTER (WHERE w.id IS NOT NULL), ARRAY[]::uuid[]) AS workspace_ids,
                  CASE WHEN bool_or(wm.status = 'active') THEN 'active'
                       WHEN bool_or(wm.status = 'suspended') THEN 'suspended'
                       ELSE tm.status::text END AS status,
                  EXISTS (
                    SELECT 1 FROM mfa_factors mf
                    WHERE mf.principal_id = u.principal_id AND mf.status = 'active'
                  ) AS mfa_enabled,
                  NULL::timestamptz AS last_seen_at,
                  u.created_at
                FROM tenant_memberships tm
                INNER JOIN users u ON u.principal_id = tm.principal_id
                LEFT JOIN (
                  workspace_memberships wm
                  INNER JOIN workspaces w ON w.id = wm.workspace_id AND w.tenant_id = $1
                ) ON wm.principal_id = u.principal_id AND wm.status IN ('active', 'suspended')
                LEFT JOIN tenant_break_glass_accounts tbga
                  ON tbga.tenant_id = tm.tenant_id
                 AND tbga.principal_id = tm.principal_id
                 AND tbga.revoked_at IS NULL
                WHERE tm.tenant_id = $1
                GROUP BY u.principal_id, u.email, u.firstname, u.lastname, u.username, u.created_at,
                  tm.status, tbga.procedure_reference, tbga.reason, tbga.created_at, tbga.last_used_at
                ORDER BY u.email ASC
                "#)
            .bind(tenant_id)
            .fetch_all(db)
            .await?)
        }
        AdminScope::Organization(org_id) => {
            Ok(sqlx::query_as::<_, EnterpriseUserRow>(r#"
                SELECT
                  u.principal_id AS id,
                  u.email,
                  COALESCE(NULLIF(concat_ws(' ', u.firstname, u.lastname), ''), u.username, u.email) AS display_name,
                  CASE MIN(CASE wm.role WHEN 'owner' THEN 0 WHEN 'admin' THEN 1 WHEN 'member' THEN 2 WHEN 'viewer' THEN 3 ELSE 4 END)
                    WHEN 0 THEN 'owner'
                    WHEN 1 THEN 'admin'
                    WHEN 2 THEN 'member'
                    ELSE 'viewer'
                  END AS role,
                  tbga.procedure_reference AS break_glass_procedure_reference,
                  tbga.reason AS break_glass_reason,
                  tbga.created_at AS break_glass_created_at,
                  tbga.last_used_at AS break_glass_last_used_at,
                  COALESCE(array_agg(DISTINCT w.id) FILTER (WHERE w.id IS NOT NULL), ARRAY[]::uuid[]) AS workspace_ids,
                  CASE WHEN bool_or(wm.status = 'active') THEN 'active'
                       WHEN bool_or(wm.status = 'suspended') THEN 'suspended'
                       ELSE tm.status::text END AS status,
                  EXISTS (
                    SELECT 1 FROM mfa_factors mf
                    WHERE mf.principal_id = u.principal_id AND mf.status = 'active'
                  ) AS mfa_enabled,
                  NULL::timestamptz AS last_seen_at,
                  u.created_at
                FROM tenant_memberships tm
                INNER JOIN users u ON u.principal_id = tm.principal_id
                INNER JOIN organization_memberships om ON om.principal_id = tm.principal_id AND om.organization_id = $2
                LEFT JOIN (
                  workspace_memberships wm
                  INNER JOIN workspaces w ON w.id = wm.workspace_id AND w.tenant_id = $1 AND w.organization_id = $2
                ) ON wm.principal_id = u.principal_id AND wm.status IN ('active', 'suspended')
                LEFT JOIN tenant_break_glass_accounts tbga
                  ON tbga.tenant_id = tm.tenant_id
                 AND tbga.principal_id = tm.principal_id
                 AND tbga.revoked_at IS NULL
                WHERE tm.tenant_id = $1
                GROUP BY u.principal_id, u.email, u.firstname, u.lastname, u.username, u.created_at,
                  tm.status, tbga.procedure_reference, tbga.reason, tbga.created_at, tbga.last_used_at
                ORDER BY u.email ASC
                "#)
            .bind(tenant_id)
            .bind(org_id)
            .fetch_all(db)
            .await?)
        }
    }
}
```

- [ ] **Step 2: Update `list_invitations` to take `scope: AdminScope`**
Modify `list_invitations` query to scope invitations to workspaces belonging to the organization:
```rust
pub async fn list_invitations(
    db: &PgPool,
    tenant_id: Uuid,
    scope: AdminScope,
) -> Result<Vec<EnterpriseInvitationRow>, AppError> {
    match scope {
        AdminScope::Tenant => {
            Ok(sqlx::query_as::<_, EnterpriseInvitationRow>(
                r#"
                SELECT wi.id, wi.email, wi.role::text AS role, ARRAY[wi.workspace_id] AS workspace_ids,
                  wi.status::text AS status, wi.created_at AS invited_at, wi.expires_at
                FROM workspace_invitations wi
                INNER JOIN workspaces w ON w.id = wi.workspace_id
                WHERE w.tenant_id = $1 AND wi.status IN ('pending', 'accepted')
                ORDER BY wi.created_at DESC
                "#,
            )
            .bind(tenant_id)
            .fetch_all(db)
            .await?)
        }
        AdminScope::Organization(org_id) => {
            Ok(sqlx::query_as::<_, EnterpriseInvitationRow>(
                r#"
                SELECT wi.id, wi.email, wi.role::text AS role, ARRAY[wi.workspace_id] AS workspace_ids,
                  wi.status::text AS status, wi.created_at AS invited_at, wi.expires_at
                FROM workspace_invitations wi
                INNER JOIN workspaces w ON w.id = wi.workspace_id
                WHERE w.tenant_id = $1 AND w.organization_id = $2 AND wi.status IN ('pending', 'accepted')
                ORDER BY wi.created_at DESC
                "#,
            )
            .bind(tenant_id)
            .bind(org_id)
            .fetch_all(db)
            .await?)
        }
    }
}
```

- [ ] **Step 3: Update `list_workspaces` to take `scope: AdminScope`**
Modify `list_workspaces` query to only return workspaces in the organization:
```rust
pub async fn list_workspaces(
    db: &PgPool,
    tenant_id: Uuid,
    scope: AdminScope,
) -> Result<Vec<WorkspaceSummaryRow>, AppError> {
    match scope {
        AdminScope::Tenant => {
            Ok(sqlx::query_as::<_, WorkspaceSummaryRow>(
                r#"
                SELECT w.id, w.name, w.workspace_type::text AS workspace_type,
                  NULL::text AS data_region,
                  COUNT(wm.principal_id) FILTER (WHERE wm.status = 'active') AS member_count,
                  COALESCE(qu.used_storage_bytes, 0)::bigint AS storage_used_bytes,
                  w.created_at
                FROM workspaces w
                LEFT JOIN workspace_memberships wm ON wm.workspace_id = w.id
                LEFT JOIN quota_usage qu ON qu.workspace_id = w.id
                WHERE w.tenant_id = $1
                GROUP BY w.id, qu.used_storage_bytes
                ORDER BY w.created_at DESC
                "#,
            )
            .bind(tenant_id)
            .fetch_all(db)
            .await?)
        }
        AdminScope::Organization(org_id) => {
            Ok(sqlx::query_as::<_, WorkspaceSummaryRow>(
                r#"
                SELECT w.id, w.name, w.workspace_type::text AS workspace_type,
                  NULL::text AS data_region,
                  COUNT(wm.principal_id) FILTER (WHERE wm.status = 'active') AS member_count,
                  COALESCE(qu.used_storage_bytes, 0)::bigint AS storage_used_bytes,
                  w.created_at
                FROM workspaces w
                LEFT JOIN workspace_memberships wm ON wm.workspace_id = w.id
                LEFT JOIN quota_usage qu ON qu.workspace_id = w.id
                WHERE w.tenant_id = $1 AND w.organization_id = $2
                GROUP BY w.id, qu.used_storage_bytes
                ORDER BY w.created_at DESC
                "#,
            )
            .bind(tenant_id)
            .bind(org_id)
            .fetch_all(db)
            .await?)
        }
    }
}
```

- [ ] **Step 4: Update `list_audit_events` to take `scope: AdminScope`**
Modify `list_audit_events` to filter events to workspaces belonging to the organization:
```rust
pub async fn list_audit_events(
    db: &PgPool,
    tenant_id: Uuid,
    scope: AdminScope,
    limit: i64,
) -> Result<Vec<AuditEventRow>, AppError> {
    match scope {
        AdminScope::Tenant => {
            Ok(sqlx::query_as::<_, AuditEventRow>(
                r#"
                SELECT ae.id, ae.action AS event_type, ae.actor_principal_id AS actor_id,
                  u.email AS actor_email, ae.target_type, ae.target_id, ae.metadata, ae.created_at
                FROM audit_events ae
                LEFT JOIN users u ON u.principal_id = ae.actor_principal_id
                WHERE ae.tenant_id = $1
                ORDER BY ae.created_at DESC, ae.id DESC
                LIMIT $2
                "#,
            )
            .bind(tenant_id)
            .bind(limit)
            .fetch_all(db)
            .await?)
        }
        AdminScope::Organization(org_id) => {
            Ok(sqlx::query_as::<_, AuditEventRow>(
                r#"
                SELECT ae.id, ae.action AS event_type, ae.actor_principal_id AS actor_id,
                  u.email AS actor_email, ae.target_type, ae.target_id, ae.metadata, ae.created_at
                FROM audit_events ae
                LEFT JOIN users u ON u.principal_id = ae.actor_principal_id
                WHERE ae.tenant_id = $1
                  AND ae.workspace_id IN (SELECT id FROM workspaces WHERE organization_id = $3)
                ORDER BY ae.created_at DESC, ae.id DESC
                LIMIT $2
                "#,
            )
            .bind(tenant_id)
            .bind(limit)
            .bind(org_id)
            .fetch_all(db)
            .await?)
        }
    }
}
```

- [ ] **Step 5: Update `usage_metrics` to take `scope: AdminScope`**
```rust
pub async fn usage_metrics(
    db: &PgPool,
    tenant_id: Uuid,
    scope: AdminScope,
) -> Result<(i64, i64, i64), AppError> {
    match scope {
        AdminScope::Tenant => {
            Ok(sqlx::query_as::<_, (i64, i64, i64)>(
                r#"
                SELECT COUNT(DISTINCT w.id)::bigint, COUNT(DISTINCT wm.principal_id)::bigint,
                  COALESCE(SUM(qu.used_storage_bytes), 0)::bigint
                FROM workspaces w
                LEFT JOIN workspace_memberships wm ON wm.workspace_id = w.id AND wm.status = 'active'
                LEFT JOIN quota_usage qu ON qu.workspace_id = w.id
                WHERE w.tenant_id = $1
                "#,
            )
            .bind(tenant_id)
            .fetch_one(db)
            .await?)
        }
        AdminScope::Organization(org_id) => {
            Ok(sqlx::query_as::<_, (i64, i64, i64)>(
                r#"
                SELECT COUNT(DISTINCT w.id)::bigint, COUNT(DISTINCT wm.principal_id)::bigint,
                  COALESCE(SUM(qu.used_storage_bytes), 0)::bigint
                FROM workspaces w
                LEFT JOIN workspace_memberships wm ON wm.workspace_id = w.id AND wm.status = 'active'
                LEFT JOIN quota_usage qu ON qu.workspace_id = w.id
                WHERE w.tenant_id = $1 AND w.organization_id = $2
                "#,
            )
            .bind(tenant_id)
            .bind(org_id)
            .fetch_one(db)
            .await?)
        }
    }
}
```

- [ ] **Step 6: Update `apps/account-service/src/identity.domains.enterprise.db.rs` exports**
Make sure the exported function signatures in `identity.domains.enterprise.db.rs` match the new database read signatures:
```rust
pub use reads::{
    actor_access, billing_summary, list_audit_events, list_developers, list_invitations,
    list_invoices, list_policies, list_users, list_workspaces, mfa_policy, security_summary,
    session_policy, usage_metrics,
};
```

- [ ] **Step 7: Commit**
```bash
git add apps/account-service/src/identity.domains.enterprise.db.reads.rs apps/account-service/src/identity.domains.enterprise.db.rs
git commit -m "feat(enterprise): scope reads queries using AdminScope"
```

---

### Task 5: Add Organization Scoping to Database Writes

**Files:**
- Modify: `apps/account-service/src/identity.domains.enterprise.db.writes.rs`

- [ ] **Step 1: Update `ensure_workspaces_belong` to take `scope: AdminScope`**
Modify `ensure_workspaces_belong` in `apps/account-service/src/identity.domains.enterprise.db.writes.rs` to validate that the workspaces are inside the organization:
```rust
pub async fn ensure_workspaces_belong(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    workspace_ids: &[Uuid],
    scope: AdminScope,
) -> Result<(), AppError> {
    let count = match scope {
        AdminScope::Tenant => {
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM workspaces WHERE tenant_id = $1 AND id = ANY($2)",
            )
            .bind(tenant_id)
            .bind(workspace_ids)
            .fetch_one(&mut **tx)
            .await?
        }
        AdminScope::Organization(org_id) => {
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM workspaces WHERE tenant_id = $1 AND organization_id = $2 AND id = ANY($3)",
            )
            .bind(tenant_id)
            .bind(org_id)
            .bind(workspace_ids)
            .fetch_one(&mut **tx)
            .await?
        }
    };
    if count != workspace_ids.len() as i64 {
        return Err(AppError::bad_request(
            "invalid_workspace_scope",
            "All workspace IDs must belong to the active tenant/organization scope.",
        ));
    }
    Ok(())
}
```

- [ ] **Step 2: Commit**
```bash
git add apps/account-service/src/identity.domains.enterprise.db.writes.rs
git commit -m "feat(enterprise): scope ensure_workspaces_belong query to AdminScope"
```

---

### Task 6: Scoping reads at the Service Layer

**Files:**
- Modify: `apps/account-service/src/identity.domains.enterprise.service.reads.rs`

- [ ] **Step 1: Resolve scope and pass to readers in all read services**
Modify `get_context`, `get_overview`, `list_users`, `list_workspaces`, and `list_audit_events` to resolve the `AdminScope` and pass it to db query functions:
For example, in `get_context`:
```rust
pub async fn get_context(
    db: &Database,
    redis: &nvbes_redis::RedisPool,
    auth: &AuthContext,
    tenant_id: Uuid,
) -> Result<EnterpriseContextResponse, AppError> {
    let scope = crate::domains::authz::service::resolve_admin_scope(db, auth, tenant_id, auth.organization_id).await?;
    let access = require_actor_access(db, auth, tenant_id, scope).await?;
    // ...
```
And in `list_users`:
```rust
pub async fn list_users(
    db: &Database,
    auth: &AuthContext,
    tenant_id: Uuid,
) -> Result<EnterpriseUsersResponse, AppError> {
    let scope = crate::domains::authz::service::resolve_admin_scope(db, auth, tenant_id, auth.organization_id).await?;
    Ok(EnterpriseUsersResponse {
        users: db::list_users(db, tenant_id, scope)
            .await?
            .into_iter()
            .map(db::EnterpriseUserRow::into_view)
            .collect(),
        invitations: db::list_invitations(db, tenant_id, scope)
            .await?
            .into_iter()
            .map(db::EnterpriseInvitationRow::into_view)
            .collect(),
        // ...
```
And in `list_workspaces`:
```rust
pub async fn list_workspaces(
    db: &Database,
    auth: &AuthContext,
    tenant_id: Uuid,
) -> Result<EnterpriseWorkspacesResponse, AppError> {
    let scope = crate::domains::authz::service::resolve_admin_scope(db, auth, tenant_id, auth.organization_id).await?;
    Ok(EnterpriseWorkspacesResponse {
        workspaces: db::list_workspaces(db, tenant_id, scope)
            .await?
            ...
```
And in `list_audit_events`:
```rust
pub async fn list_audit_events(
    db: &Database,
    auth: &AuthContext,
    tenant_id: Uuid,
) -> Result<EnterpriseAuditEventsResponse, AppError> {
    let scope = crate::domains::authz::service::resolve_admin_scope(db, auth, tenant_id, auth.organization_id).await?;
    Ok(EnterpriseAuditEventsResponse {
        events: db::list_audit_events(db, tenant_id, scope, 50)
            .await?
            ...
```

- [ ] **Step 2: Restrict tenant-wide endpoints for organization admins**
In `list_developers`, `list_policies`, `get_security`, `get_billing`, `get_usage`, enforce that scope must be `AdminScope::Tenant`. Return forbidden error if organization scoped:
```rust
    let scope = crate::domains::authz::service::resolve_admin_scope(db, auth, tenant_id, auth.organization_id).await?;
    if !matches!(scope, AdminScope::Tenant) {
        return Err(AppError::forbidden(
            "tenant_scope_required",
            "This action requires tenant-wide administrative privileges.",
        ));
    }
```

- [ ] **Step 3: Commit**
```bash
git add apps/account-service/src/identity.domains.enterprise.service.reads.rs
git commit -m "feat(enterprise): resolve scope and enforce boundary in service reads"
```

---

### Task 7: Scoping Mutations at the Service Layer

**Files:**
- Modify: `apps/account-service/src/identity.domains.enterprise.service.mutations.rs`
- Modify: `apps/account-service/src/identity.domains.enterprise.service.user_mutations.rs`

- [ ] **Step 1: Scoping invitations in `create_invitations`**
Update `create_invitations` to resolve the `AdminScope` and pass it to validations:
```rust
pub async fn create_invitations(
    db: &Database,
    redis: &nvbes_redis::RedisPool,
    auth: &AuthContext,
    tenant_id: Uuid,
    input: EnterpriseInvitationInput,
) -> Result<EnterpriseInvitationsResponse, AppError> {
    let scope = crate::domains::authz::service::resolve_admin_scope(db, auth, tenant_id, auth.organization_id).await?;
    let actor_access = ensure_member_manager(db, redis, auth, tenant_id, scope).await?;
    ensure_owner_role_allowed(&actor_access, policy::role_as_db(&input.role))?;
    if input.workspace_ids.is_empty() || input.emails.is_empty() {
        return Err(AppError::bad_request(
            "validation_failed",
            "At least one email and one workspace ID are required.",
        ));
    }

    let mut tx = db.begin().await?;
    db::ensure_workspaces_belong(&mut tx, tenant_id, &input.workspace_ids, scope).await?;
    // ...
```

- [ ] **Step 2: Restrict developer credentials revocation in `revoke_developer_secret`**
Block organization-scoped admins from revoking developer credentials:
```rust
pub async fn revoke_developer_secret(
    db: &Database,
    redis: &nvbes_redis::RedisPool,
    auth: &AuthContext,
    tenant_id: Uuid,
    version_id: Uuid,
) -> Result<EnterpriseDevelopersResponse, AppError> {
    let scope = crate::domains::authz::service::resolve_admin_scope(db, auth, tenant_id, auth.organization_id).await?;
    if !matches!(scope, AdminScope::Tenant) {
        return Err(AppError::forbidden(
            "tenant_scope_required",
            "This action requires tenant-wide administrative privileges.",
        ));
    }
    // ...
```

- [ ] **Step 3: Implement `ensure_user_in_organization` helper**
Add `ensure_user_in_organization` to `apps/account-service/src/identity.domains.enterprise.service.user_mutations.rs`:
```rust
async fn ensure_user_in_organization(
    db: &Database,
    user_id: Uuid,
    org_id: Uuid,
) -> Result<(), AppError> {
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (SELECT 1 FROM organization_memberships WHERE organization_id = $1 AND principal_id = $2 AND status = 'active')",
    )
    .bind(org_id)
    .bind(user_id)
    .fetch_one(db)
    .await?;

    if !exists {
        return Err(AppError::forbidden(
            "user_not_in_organization",
            "This user is not a member of your organization.",
        ));
    }
    Ok(())
}
```

- [ ] **Step 4: Scoping user access changes in `update_user_access`, `suspend_user`, and `reactivate_user`**
Modify all three functions in `apps/account-service/src/identity.domains.enterprise.service.user_mutations.rs` to resolve the `AdminScope`. If organization-scoped, check `ensure_user_in_organization` and scope workspace checks:
```rust
pub async fn update_user_access(
    db: &Database,
    redis: &nvbes_redis::RedisPool,
    auth: &AuthContext,
    tenant_id: Uuid,
    user_id: Uuid,
    input: EnterpriseAccessUpdateInput,
) -> Result<EnterpriseAccessUpdateResponse, AppError> {
    let scope = crate::domains::authz::service::resolve_admin_scope(db, auth, tenant_id, auth.organization_id).await?;
    let actor_access = ensure_member_manager(db, redis, auth, tenant_id, scope).await?;
    ensure_owner_role_allowed(&actor_access, policy::role_as_db(&input.role))?;

    if let AdminScope::Organization(org_id) = scope {
        ensure_user_in_organization(db, user_id, org_id).await?;
    }

    if input.workspace_ids.is_empty() {
        return Err(AppError::bad_request(
            "validation_failed",
            "At least one workspace ID is required.",
        ));
    }
    let mut tx = db.begin().await?;
    db::lock_tenant_owner_changes(&mut tx, tenant_id).await?;
    db::ensure_workspaces_belong(&mut tx, tenant_id, &input.workspace_ids, scope).await?;
    // ...
```
Do the same for `suspend_user` and `reactivate_user` (and update the calls to `fetch_user_view(db, tenant_id, user_id, scope)`).

- [ ] **Step 5: Scope Policy mutations**
In `apps/account-service/src/identity.domains.enterprise.service.policy_mutations.rs`, make sure policy mutations (`update_mfa_policy`, `update_session_policy`) check and block organization scoped admins:
```rust
    let scope = crate::domains::authz::service::resolve_admin_scope(db, auth, tenant_id, auth.organization_id).await?;
    if !matches!(scope, AdminScope::Tenant) {
        return Err(AppError::forbidden(
            "tenant_scope_required",
            "This action requires tenant-wide administrative privileges.",
        ));
    }
```
In `apps/account-service/src/identity.domains.enterprise.policy_simulation.rs`, verify `simulate_policy_decision` also blocks organization scoped admins.

- [ ] **Step 6: Scope Access reviews**
In `apps/account-service/src/identity.domains.enterprise.access_reviews.service.rs` and `apps/account-service/src/identity.domains.enterprise.access_reviews.service.schedules.rs`, block access review administration for organization-scoped admins.

- [ ] **Step 7: Commit**
```bash
git add apps/account-service/src/identity.domains.enterprise.service.mutations.rs apps/account-service/src/identity.domains.enterprise.service.user_mutations.rs apps/account-service/src/identity.domains.enterprise.service.policy_mutations.rs apps/account-service/src/identity.domains.enterprise.policy_simulation.rs apps/account-service/src/identity.domains.enterprise.access_reviews.service.rs apps/account-service/src/identity.domains.enterprise.access_reviews.service.schedules.rs
git commit -m "feat(enterprise): enforce AdminScope in all mutations and access review endpoints"
```

---

### Task 8: Verification and Testing

- [ ] **Step 1: Run complete workspace compilation**
Run: `rtk cargo check --workspace`
Expected: PASS

- [ ] **Step 2: Add integration tests for organization scoping**
Create `apps/account-service/src/identity.domains.enterprise.tests.rs` (or extend an existing test) to setup:
1. A tenant with two organizations (Org A and Org B), with some workspaces in each.
2. Users: Tenant Admin, Admin of Org A, Member of Org A.
3. Verify that Admin of Org A:
   - Can list workspaces/users in Org A, but NOT in Org B.
   - Can invite members to workspaces in Org A, but NOT Org B.
   - Gets forbidden on Billing/Security/Developers views.
   - Cannot view or modify user access in Org B.

- [ ] **Step 3: Run all tests in the workspace**
Run: `rtk cargo test --workspace`
Expected: PASS
