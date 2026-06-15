use chrono::{DateTime, Utc};
use nvbes_tenancy::{InheritedPolicyLayer, resolve_inherited_policy};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::domains::{
    authz::{WorkspaceRole, parse_role},
    enterprise::policy_simulation::types::EnterprisePolicySimulationSubject,
};
use crate::http::error::AppError;

#[derive(Debug)]
pub(super) struct SimulationSubject {
    pub(super) principal_id: Uuid,
    pub(super) subject_type: String,
    pub(super) subject_id: String,
    pub(super) subject_label: String,
    pub(super) email_verified_at: Option<DateTime<Utc>>,
}

#[derive(Debug)]
pub(super) struct SimulationAccess {
    pub(super) role: WorkspaceRole,
    pub(super) member_share_links_enabled: bool,
}

pub(super) async fn resolve_subject(
    db: &PgPool,
    tenant_id: Uuid,
    workspace_id: Uuid,
    subject: EnterprisePolicySimulationSubject,
) -> Result<SimulationSubject, AppError> {
    match subject {
        EnterprisePolicySimulationSubject::User { user_id } => {
            resolve_user_subject(db, tenant_id, user_id).await
        }
        EnterprisePolicySimulationSubject::Client { client_id } => {
            resolve_client_subject(db, tenant_id, workspace_id, &client_id).await
        }
    }
}

pub(super) async fn load_simulated_access(
    db: &PgPool,
    tenant_id: Uuid,
    workspace_id: Uuid,
    principal_id: Uuid,
) -> Result<Option<SimulationAccess>, AppError> {
    let row = sqlx::query(
        r#"
        SELECT
          wm.role::text AS role,
          sp.member_can_create_share_links AS system_member_can_create_share_links,
          tp.member_can_create_share_links AS tenant_member_can_create_share_links,
          op.member_can_create_share_links AS organization_member_can_create_share_links,
          wp.member_can_create_share_links AS workspace_member_can_create_share_links
        FROM workspaces w
        INNER JOIN workspace_memberships wm ON wm.workspace_id = w.id
        INNER JOIN workspace_policies wp ON wp.workspace_id = w.id
        INNER JOIN system_policies sp ON sp.id = TRUE
        LEFT JOIN tenant_policies tp ON tp.tenant_id = w.tenant_id
        LEFT JOIN organization_policies op ON op.organization_id = w.organization_id
        WHERE w.id = $1
          AND w.tenant_id = $2
          AND wm.principal_id = $3
          AND wm.status = 'active'
        "#,
    )
    .bind(workspace_id)
    .bind(tenant_id)
    .bind(principal_id)
    .fetch_optional(db)
    .await?;

    row.map(|row| {
        let effective_policy = resolve_inherited_policy([
            InheritedPolicyLayer::system()
                .with_member_share_links(row.get("system_member_can_create_share_links")),
            optional_member_share_layer(
                InheritedPolicyLayer::tenant(),
                row.get("tenant_member_can_create_share_links"),
            ),
            optional_member_share_layer(
                InheritedPolicyLayer::organization(),
                row.get("organization_member_can_create_share_links"),
            ),
            InheritedPolicyLayer::workspace()
                .with_member_share_links(row.get("workspace_member_can_create_share_links")),
        ]);
        Ok(SimulationAccess {
            role: parse_role(row.get::<String, _>("role").as_str())?,
            member_share_links_enabled: effective_policy.member_can_create_share_links,
        })
    })
    .transpose()
}

async fn resolve_user_subject(
    db: &PgPool,
    tenant_id: Uuid,
    user_id: Uuid,
) -> Result<SimulationSubject, AppError> {
    let row = sqlx::query(
        r#"
        SELECT
          u.principal_id,
          COALESCE(NULLIF(concat_ws(' ', u.firstname, u.lastname), ''), u.username, u.email) AS label,
          u.email_verified_at
        FROM tenant_memberships tm
        INNER JOIN users u ON u.principal_id = tm.principal_id
        WHERE tm.tenant_id = $1
          AND tm.principal_id = $2
          AND tm.status = 'active'
        "#,
    )
    .bind(tenant_id)
    .bind(user_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::not_found("subject_not_found", "User subject not found."))?;

    Ok(SimulationSubject {
        principal_id: row.get("principal_id"),
        subject_type: "user".to_string(),
        subject_id: user_id.to_string(),
        subject_label: row.get::<String, _>("label"),
        email_verified_at: row.get("email_verified_at"),
    })
}

async fn resolve_client_subject(
    db: &PgPool,
    tenant_id: Uuid,
    workspace_id: Uuid,
    client_id: &str,
) -> Result<SimulationSubject, AppError> {
    let row = sqlx::query(
        r#"
        SELECT sa.principal_id, sa.name
        FROM service_accounts sa
        INNER JOIN principals p ON p.id = sa.principal_id
        WHERE sa.tenant_id = $1
          AND sa.workspace_id = $2
          AND sa.client_id = $3
          AND p.status = 'active'
        "#,
    )
    .bind(tenant_id)
    .bind(workspace_id)
    .bind(client_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::not_found("subject_not_found", "Client subject not found."))?;

    Ok(SimulationSubject {
        principal_id: row.get("principal_id"),
        subject_type: "client".to_string(),
        subject_id: client_id.to_string(),
        subject_label: row.get("name"),
        email_verified_at: Some(Utc::now()),
    })
}

fn optional_member_share_layer(
    mut layer: InheritedPolicyLayer,
    member_can_create_share_links: Option<bool>,
) -> InheritedPolicyLayer {
    layer.member_can_create_share_links = member_can_create_share_links;
    layer
}
