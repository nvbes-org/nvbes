#[path = "identity.domains.enterprise.policy_simulation.db.rs"]
mod db;
#[path = "identity.domains.enterprise.policy_simulation.types.rs"]
pub mod types;

use sqlx::PgPool;
use uuid::Uuid;

use crate::domains::{
    authz::{AdminScope, parse_action, parse_role, resolve_admin_scope},
    enterprise::{grpc as enterprise_grpc, policy},
};
use crate::http::error::AppError;
use crate::http::middleware::jwt::AuthContext;

pub use types::{
    EnterprisePolicySimulationDecision, EnterprisePolicySimulationInput,
    EnterprisePolicySimulationResponse,
};

pub async fn simulate_policy(
    db: &PgPool,
    auth: &AuthContext,
    tenant_id: Uuid,
    input: EnterprisePolicySimulationInput,
) -> Result<EnterprisePolicySimulationResponse, AppError> {
    ensure_policy_manager(db, auth, tenant_id).await?;
    let workspace_id = input.workspace_id;
    let action = parse_action(&input.action)?;
    let target_role = input
        .resource
        .as_ref()
        .and_then(|resource| resource.target_role.as_deref())
        .map(parse_role)
        .transpose()?;
    let resource = input.resource.unwrap_or_default();
    let subject = db::resolve_subject(db, tenant_id, workspace_id, input.subject).await?;
    let access =
        db::load_simulated_access(db, tenant_id, workspace_id, subject.principal_id).await?;
    let role = access.as_ref().map(|access| access.role.to_string());
    let member_share_links_enabled = resource.member_share_links_enabled
        || access
            .as_ref()
            .is_some_and(|access| access.member_share_links_enabled);
    let decision = enterprise_grpc::evaluate_policy_simulation(
        tenant_id,
        auth.user_id,
        enterprise_grpc::PolicySimulationEvaluationInput {
            workspace_id,
            subject_principal_id: subject.principal_id,
            subject_type: subject.subject_type,
            subject_id: subject.subject_id,
            subject_label: subject.subject_label,
            email_verified: subject.email_verified_at.is_some(),
            action: action.as_str().to_string(),
            role,
            owns_resource: resource.owns_resource,
            member_share_links_enabled,
            target_role: target_role.map(|role| role.to_string()),
        },
    )
    .await?;

    Ok(EnterprisePolicySimulationResponse { decision })
}

async fn ensure_policy_manager(
    db: &PgPool,
    auth: &AuthContext,
    tenant_id: Uuid,
) -> Result<(), AppError> {
    let scope = resolve_admin_scope(db, auth, tenant_id, auth.organization_id).await?;
    if !matches!(scope, AdminScope::Tenant) {
        return Err(AppError::forbidden(
            "tenant_scope_required",
            "This action requires tenant-wide administrative privileges.",
        ));
    }
    let row = crate::domains::enterprise::db::actor_access(db, tenant_id, auth.user_id)
        .await?
        .ok_or_else(|| {
            AppError::forbidden(
                "tenant_management_denied",
                "You do not have permission to manage this tenant.",
            )
        })?;
    let role = policy::role_from_db(&row.role);
    let grants = policy::grant_names(&policy::grants_for_role(&role));
    if !policy::can_manage_policies(&row.role, &grants) {
        return Err(AppError::forbidden(
            "policies_grant_required",
            "Policies access is required.",
        ));
    }
    Ok(())
}
