use chrono::{Duration, Utc};
use sqlx::{Postgres, Transaction};
use tonic::Status;
use uuid::Uuid;

use crate::grpc::{
    pb::nvbes::enterprise::v1 as enterprise,
    service_status::{parse_uuid, sql_status},
    user_access::AccessScope,
};

#[cfg(test)]
#[path = "enterprise.grpc.invitations.contract_tests.rs"]
mod contract_tests;

pub async fn create_invitations(
    pool: &sqlx::PgPool,
    tenant_id: Uuid,
    actor_id: Uuid,
    request: enterprise::CreateInvitationsRequest,
) -> Result<enterprise::CreateInvitationsResponse, Status> {
    let scope = AccessScope::from_request(&request.scope, &request.organization_id)?;
    let role = validate_role(&request.role)?;
    let workspace_ids = parse_workspace_ids(&request.workspace_ids)?;
    if request.emails.is_empty() {
        return Err(Status::invalid_argument("at least one email is required"));
    }

    let mut tx = pool.begin().await.map_err(sql_status)?;
    ensure_workspaces_belong(&mut tx, tenant_id, &workspace_ids, scope).await?;
    let mut invitations = Vec::new();
    for raw_email in request.emails {
        let email = nvbes_core::auth::normalize_email(&raw_email);
        nvbes_core::auth::validate_email(&email)
            .map_err(|error| Status::invalid_argument(error.message))?;
        for workspace_id in &workspace_ids {
            ensure_no_pending_invitation(&mut tx, tenant_id, *workspace_id, &email).await?;
            let invitation =
                insert_invitation(&mut tx, tenant_id, *workspace_id, &email, role).await?;
            insert_invitation_audit(&mut tx, tenant_id, actor_id, &invitation).await?;
            invitations.push(invitation);
        }
    }
    tx.commit().await.map_err(sql_status)?;
    Ok(enterprise::CreateInvitationsResponse { invitations })
}

pub(crate) async fn ensure_workspaces_belong(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    workspace_ids: &[Uuid],
    scope: AccessScope,
) -> Result<(), Status> {
    let count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM workspaces
        WHERE tenant_id = $1
          AND id = ANY($2)
          AND ($3::uuid IS NULL OR organization_id = $3)
        "#,
    )
    .bind(tenant_id)
    .bind(workspace_ids)
    .bind(scope.organization_id())
    .fetch_one(&mut **tx)
    .await
    .map_err(sql_status)?;
    if count != workspace_ids.len() as i64 {
        return Err(Status::invalid_argument(
            "all workspace IDs must belong to the requested scope",
        ));
    }
    Ok(())
}

fn validate_role(role: &str) -> Result<&str, Status> {
    match role.trim() {
        "owner" | "admin" | "member" | "viewer" => Ok(role.trim()),
        _ => Err(Status::invalid_argument(
            "role must be owner, admin, member, or viewer",
        )),
    }
}

fn parse_workspace_ids(values: &[String]) -> Result<Vec<Uuid>, Status> {
    if values.is_empty() {
        return Err(Status::invalid_argument(
            "at least one workspace_id is required",
        ));
    }
    values
        .iter()
        .map(|value| parse_uuid(value, "workspace_id"))
        .collect()
}

async fn ensure_no_pending_invitation(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    workspace_id: Uuid,
    email: &str,
) -> Result<(), Status> {
    let count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM workspace_invitations wi
        INNER JOIN workspaces w ON w.id = wi.workspace_id
        WHERE w.tenant_id = $1
          AND wi.workspace_id = $2
          AND lower(wi.email) = $3
          AND wi.status = 'pending'
          AND wi.expires_at > NOW()
        "#,
    )
    .bind(tenant_id)
    .bind(workspace_id)
    .bind(email)
    .fetch_one(&mut **tx)
    .await
    .map_err(sql_status)?;
    if count > 0 {
        Err(Status::already_exists(
            "a pending invitation already exists for this email and workspace",
        ))
    } else {
        Ok(())
    }
}

async fn insert_invitation(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    workspace_id: Uuid,
    email: &str,
    role: &str,
) -> Result<enterprise::Invitation, Status> {
    let token = nvbes_core::auth::generate_token("gxi");
    let token_hash = nvbes_core::auth::token_hash(&token);
    let expires_at = Utc::now() + Duration::days(7);
    let row = sqlx::query_as::<_, InvitationRow>(
        r#"
        INSERT INTO workspace_invitations (workspace_id, email, role, token_hash, expires_at)
        SELECT w.id, $3, $4::workspace_member_role, $5, $6
        FROM workspaces w
        WHERE w.id = $1 AND w.tenant_id = $2
        RETURNING id, email, role::text AS role, ARRAY[workspace_id] AS workspace_ids,
          status::text AS status, created_at AS invited_at, expires_at
        "#,
    )
    .bind(workspace_id)
    .bind(tenant_id)
    .bind(email)
    .bind(role)
    .bind(token_hash)
    .bind(expires_at)
    .fetch_one(&mut **tx)
    .await
    .map_err(sql_status)?;
    Ok(row.into_proto())
}

async fn insert_invitation_audit(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    actor_id: Uuid,
    invitation: &enterprise::Invitation,
) -> Result<(), Status> {
    let workspace_id = parse_uuid(&invitation.workspace_ids[0], "workspace_id")?;
    let invitation_id = parse_uuid(&invitation.invitation_id, "invitation_id")?;
    sqlx::query(
        r#"
        INSERT INTO audit_events (
          tenant_id, actor_principal_id, action, target_type, target_id,
          metadata, event_hash, created_at
        )
        VALUES ($1, $2, 'enterprise.member.invited', 'workspace_invitation', $3, $4, gen_random_uuid()::text, NOW())
        "#,
    )
    .bind(tenant_id)
    .bind(actor_id)
    .bind(invitation_id)
    .bind(serde_json::json!({
        "email": invitation.email,
        "role": invitation.role,
        "workspace_id": workspace_id
    }))
    .execute(&mut **tx)
    .await
    .map_err(sql_status)?;
    Ok(())
}

#[derive(sqlx::FromRow)]
struct InvitationRow {
    id: Uuid,
    email: String,
    role: String,
    workspace_ids: Vec<Uuid>,
    status: String,
    invited_at: chrono::DateTime<Utc>,
    expires_at: Option<chrono::DateTime<Utc>>,
}

impl InvitationRow {
    fn into_proto(self) -> enterprise::Invitation {
        enterprise::Invitation {
            invitation_id: self.id.to_string(),
            email: self.email,
            role: self.role,
            workspace_ids: self
                .workspace_ids
                .into_iter()
                .map(|workspace_id| workspace_id.to_string())
                .collect(),
            status: self.status,
            invited_at: self.invited_at.to_rfc3339(),
            expires_at: self
                .expires_at
                .map(|value| value.to_rfc3339())
                .unwrap_or_default(),
        }
    }
}
