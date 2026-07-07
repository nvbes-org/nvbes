use chrono::{DateTime, Utc};
use nvbes_tenancy::{InheritedPolicyLayer, resolve_inherited_policy};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::domains::{
    authz::{WorkspaceRole, parse_role},
    cloud::workspace_port,
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
    let workspace =
        workspace_port::get_workspace(Some(tenant_id), workspace_id, principal_id).await?;
    let Some(member) =
        workspace_port::list_workspace_members(Some(tenant_id), workspace_id, principal_id)
            .await?
            .into_iter()
            .find(|member| member.principal_id == principal_id && member.active)
    else {
        return Ok(None);
    };
    let row = sqlx::query(
        r#"
        SELECT
          sp.member_can_create_share_links AS system_member_can_create_share_links,
          tp.member_can_create_share_links AS tenant_member_can_create_share_links,
          op.member_can_create_share_links AS organization_member_can_create_share_links
        FROM system_policies sp
        LEFT JOIN tenant_policies tp ON tp.tenant_id = $1
        LEFT JOIN organization_policies op ON op.organization_id = $2
        WHERE sp.id = TRUE
        "#,
    )
    .bind(tenant_id)
    .bind(workspace.organization_id)
    .fetch_one(db)
    .await?;

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
            .with_member_share_links(workspace.member_can_create_share_links),
    ]);
    Ok(Some(SimulationAccess {
        role: parse_role(&member.role)?,
        member_share_links_enabled: effective_policy.member_can_create_share_links,
    }))
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
