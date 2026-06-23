use axum::http::HeaderMap;

use crate::{
    domains::{
        auth::types::AuthPrincipalKind,
        authz::types::{ResourceContext, WorkspaceAccess, WorkspaceAction},
    },
    http::{
        error::AppError,
        request::{client_ip, user_agent},
    },
};

use super::role_policy::principal_type_label;

pub async fn record_permission_denied(
    db: &sqlx::PgPool,
    access: &WorkspaceAccess,
    action: WorkspaceAction,
    resource: ResourceContext,
    headers: &HeaderMap,
) -> Result<(), AppError> {
    let ip = client_ip(headers);
    let metadata = serde_json::json!({
      "requested_action": format!("{action:?}"),
      "role": format!("{:?}", access.role),
      "principal_id": access.auth.principal_id,
      "principal_type": principal_type_label(access.auth.principal_kind),
      "tenant_id": access.tenant_id,
      "organization_id": access.organization_id,
      "owns_resource": resource.owns_resource,
      "member_share_links_enabled": resource.member_share_links_enabled,
      "target_role": resource.target_role.map(|role| format!("{role:?}")),
      "actor": access.auth.actor.as_ref().map(|actor| serde_json::json!({
        "principal_id": actor.principal_id,
        "principal_type": match actor.principal_kind {
          AuthPrincipalKind::User => "user",
          AuthPrincipalKind::ServiceAccount => "service_account",
        },
        "tenant_id": actor.tenant_id,
        "organization_id": actor.organization_id,
        "workspace_id": actor.workspace_id,
        "role": actor.role,
        "client_id": actor.client_id
      }))
    });
    let mut tx = db.begin().await?;
    let metadata = crate::domains::audit::geo::enrich_audit_metadata_tx(
        &mut tx,
        access.workspace_id,
        ip.as_deref(),
        "permission.denied",
        metadata,
    )
    .await?;

    sqlx::query(
        r#"
        INSERT INTO audit_events (
          workspace_id,
          actor_user_id,
          actor_principal_id,
          action,
          target_type,
          target_id,
          ip,
          user_agent,
          metadata
        )
        VALUES ($1, $2, $3, 'permission.denied', 'workspace', $4, $5::inet, $6, $7)
        "#,
    )
    .bind(access.workspace_id)
    .bind(access.auth.audit_actor_user_id())
    .bind(Some(access.auth.audit_actor_principal_id()))
    .bind(access.workspace_id)
    .bind(ip.as_deref())
    .bind(user_agent(headers).as_deref())
    .bind(sqlx::types::Json(metadata))
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(())
}
