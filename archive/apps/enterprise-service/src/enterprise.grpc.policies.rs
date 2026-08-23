use chrono::Utc;
use sqlx::Row;
use tonic::Status;
use uuid::Uuid;

use crate::grpc::policy_constraints::AuthorizationConstraints;
use crate::grpc::{pb::nvbes::enterprise::v1 as enterprise, service_status::sql_status};

pub async fn policy_set(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
) -> Result<enterprise::PolicySet, Status> {
    let rows = sqlx::query(
        r#"
        SELECT
          'mfa' AS policy_kind,
          COALESCE(t.mfa_policy, 'optional') AS rules_json,
          'active' AS status,
          t.updated_at
        FROM tenants t
        WHERE t.id = $1
        UNION ALL
        SELECT
          'session' AS policy_kind,
          json_build_object('admin_session_ttl_hours', tp.admin_session_ttl_hours)::text AS rules_json,
          'active' AS status,
          tp.updated_at
        FROM tenant_policies tp
        WHERE tp.tenant_id = $1
        UNION ALL
        SELECT
          'authorization' AS policy_kind,
          tp.authorization_constraints::text AS rules_json,
          'active' AS status,
          tp.updated_at
        FROM tenant_policies tp
        WHERE tp.tenant_id = $1
        "#,
    )
    .bind(tenant_id)
    .fetch_all(db)
    .await
    .map_err(sql_status)?;

    let policies = rows
        .into_iter()
        .map(|row| {
            let policy_kind: String = row.get("policy_kind");
            let updated_at: chrono::DateTime<Utc> = row.get("updated_at");
            enterprise::EnterprisePolicy {
                policy_id: format!("{tenant_id}:{policy_kind}"),
                policy_kind,
                rules_json: row.get("rules_json"),
                status: row.get("status"),
                updated_at: updated_at.to_rfc3339(),
            }
        })
        .collect();
    Ok(enterprise::PolicySet {
        tenant_id: tenant_id.to_string(),
        policies,
    })
}

pub async fn update_policy(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
    policy_kind: &str,
    rules_json: &str,
) -> Result<enterprise::EnterprisePolicy, Status> {
    match policy_kind {
        "session" => update_session_policy(db, tenant_id, rules_json).await,
        "mfa" => update_mfa_policy(db, tenant_id, rules_json).await,
        "authorization" => {
            update_authorization_policy(db, tenant_id, "tenant", tenant_id, rules_json).await
        }
        _ => Err(Status::invalid_argument(
            "unsupported enterprise policy_kind",
        )),
    }
}

pub async fn update_scoped_policy(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
    policy_kind: &str,
    scope_type: &str,
    scope_id: Uuid,
    rules_json: &str,
) -> Result<enterprise::EnterprisePolicy, Status> {
    if policy_kind == "authorization" {
        return update_authorization_policy(db, tenant_id, scope_type, scope_id, rules_json).await;
    }
    if scope_type != "tenant" || scope_id != tenant_id {
        return Err(Status::invalid_argument(
            "mfa and session policies only support tenant scope",
        ));
    }
    update_policy(db, tenant_id, policy_kind, rules_json).await
}

pub async fn effective_authorization_constraints(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
    organization_id: Option<Uuid>,
    workspace_id: Option<Uuid>,
) -> Result<AuthorizationConstraints, Status> {
    let mut effective = AuthorizationConstraints::default();
    if let Some(value) = sqlx::query_scalar::<_, serde_json::Value>(
        "SELECT authorization_constraints FROM tenant_policies WHERE tenant_id = $1",
    )
    .bind(tenant_id)
    .fetch_optional(db)
    .await
    .map_err(sql_status)?
    {
        effective = effective.combine_with_child(&AuthorizationConstraints::from_json(&value)?);
    }
    if let Some(organization_id) = organization_id {
        if let Some(value) = sqlx::query_scalar::<_, serde_json::Value>(
            r#"
            SELECT op.authorization_constraints
            FROM organization_policies op
            INNER JOIN organizations o ON o.id = op.organization_id
            WHERE op.organization_id = $1 AND o.tenant_id = $2
            "#,
        )
        .bind(organization_id)
        .bind(tenant_id)
        .fetch_optional(db)
        .await
        .map_err(sql_status)?
        {
            effective = effective.combine_with_child(&AuthorizationConstraints::from_json(&value)?);
        }
    }
    if let Some(workspace_id) = workspace_id {
        if let Some(value) = sqlx::query_scalar::<_, serde_json::Value>(
            r#"
            SELECT wp.authorization_constraints
            FROM workspace_policies wp
            INNER JOIN workspaces w ON w.id = wp.workspace_id
            WHERE wp.workspace_id = $1 AND w.tenant_id = $2
            "#,
        )
        .bind(workspace_id)
        .bind(tenant_id)
        .fetch_optional(db)
        .await
        .map_err(sql_status)?
        {
            effective = effective.combine_with_child(&AuthorizationConstraints::from_json(&value)?);
        }
    }
    Ok(effective)
}

async fn update_authorization_policy(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
    scope_type: &str,
    scope_id: Uuid,
    rules_json: &str,
) -> Result<enterprise::EnterprisePolicy, Status> {
    let rules: serde_json::Value = serde_json::from_str(rules_json)
        .map_err(|_| Status::invalid_argument("authorization rules_json must be JSON"))?;
    AuthorizationConstraints::from_json(&rules)?;

    match scope_type {
        "tenant" if scope_id == tenant_id => {
            sqlx::query(
                r#"
                INSERT INTO tenant_policies (tenant_id, authorization_constraints)
                VALUES ($1, $2)
                ON CONFLICT (tenant_id) DO UPDATE SET
                  authorization_constraints = EXCLUDED.authorization_constraints,
                  updated_at = NOW()
                "#,
            )
            .bind(tenant_id)
            .bind(&rules)
            .execute(db)
            .await
            .map_err(sql_status)?;
        }
        "organization" => {
            let changed = sqlx::query(
                r#"
                INSERT INTO organization_policies (organization_id, authorization_constraints)
                SELECT id, $3
                FROM organizations
                WHERE id = $1 AND tenant_id = $2
                ON CONFLICT (organization_id) DO UPDATE SET
                  authorization_constraints = EXCLUDED.authorization_constraints,
                  updated_at = NOW()
                "#,
            )
            .bind(scope_id)
            .bind(tenant_id)
            .bind(&rules)
            .execute(db)
            .await
            .map_err(sql_status)?
            .rows_affected();
            ensure_scope_exists(changed)?;
        }
        "workspace" => {
            let changed = sqlx::query(
                r#"
                INSERT INTO workspace_policies (workspace_id, authorization_constraints)
                SELECT id, $3
                FROM workspaces
                WHERE id = $1 AND tenant_id = $2
                ON CONFLICT (workspace_id) DO UPDATE SET
                  authorization_constraints = EXCLUDED.authorization_constraints,
                  updated_at = NOW()
                "#,
            )
            .bind(scope_id)
            .bind(tenant_id)
            .bind(&rules)
            .execute(db)
            .await
            .map_err(sql_status)?
            .rows_affected();
            ensure_scope_exists(changed)?;
        }
        _ => {
            return Err(Status::invalid_argument(
                "authorization scope must be tenant, organization, or workspace",
            ));
        }
    }

    Ok(enterprise::EnterprisePolicy {
        policy_id: format!("{scope_type}:{scope_id}:authorization"),
        policy_kind: "authorization".to_string(),
        rules_json: rules.to_string(),
        status: "active".to_string(),
        updated_at: Utc::now().to_rfc3339(),
    })
}

fn ensure_scope_exists(rows_affected: u64) -> Result<(), Status> {
    if rows_affected == 0 {
        Err(Status::not_found(
            "authorization policy scope was not found in tenant",
        ))
    } else {
        Ok(())
    }
}

async fn update_session_policy(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
    rules_json: &str,
) -> Result<enterprise::EnterprisePolicy, Status> {
    let rules: serde_json::Value = serde_json::from_str(rules_json)
        .map_err(|_| Status::invalid_argument("session policy rules_json must be JSON"))?;
    let ttl = rules
        .get("admin_session_ttl_hours")
        .and_then(serde_json::Value::as_i64)
        .ok_or_else(|| {
            Status::invalid_argument("session policy requires admin_session_ttl_hours")
        })?;
    sqlx::query(
        r#"
        INSERT INTO tenant_policies (tenant_id, admin_session_ttl_hours)
        VALUES ($1, $2)
        ON CONFLICT (tenant_id)
        DO UPDATE SET admin_session_ttl_hours = EXCLUDED.admin_session_ttl_hours, updated_at = NOW()
        "#,
    )
    .bind(tenant_id)
    .bind(ttl as i32)
    .execute(db)
    .await
    .map_err(sql_status)?;

    Ok(enterprise::EnterprisePolicy {
        policy_id: format!("{tenant_id}:session"),
        policy_kind: "session".to_string(),
        rules_json: rules_json.to_string(),
        status: "active".to_string(),
        updated_at: Utc::now().to_rfc3339(),
    })
}

async fn update_mfa_policy(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
    rules_json: &str,
) -> Result<enterprise::EnterprisePolicy, Status> {
    let rules: serde_json::Value = serde_json::from_str(rules_json)
        .map_err(|_| Status::invalid_argument("mfa policy rules_json must be JSON"))?;
    let policy = rules
        .get("policy")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| Status::invalid_argument("mfa policy requires policy"))?;
    if !matches!(policy, "optional" | "required_admins" | "required_all") {
        return Err(Status::invalid_argument("unsupported mfa policy"));
    }

    sqlx::query("UPDATE tenants SET mfa_policy = $2, updated_at = NOW() WHERE id = $1")
        .bind(tenant_id)
        .bind(policy)
        .execute(db)
        .await
        .map_err(sql_status)?;

    Ok(enterprise::EnterprisePolicy {
        policy_id: format!("{tenant_id}:mfa"),
        policy_kind: "mfa".to_string(),
        rules_json: rules_json.to_string(),
        status: "active".to_string(),
        updated_at: Utc::now().to_rfc3339(),
    })
}
