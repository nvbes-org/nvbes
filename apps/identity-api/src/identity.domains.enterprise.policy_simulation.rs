#[path = "identity.domains.enterprise.policy_simulation.db.rs"]
mod db;
#[path = "identity.domains.enterprise.policy_simulation.types.rs"]
pub mod types;

use nvbes_core::authz::{action_requires_step_up, is_allowed};
use sqlx::PgPool;
use uuid::Uuid;

use crate::domains::{
    authz::{ResourceContext, WorkspaceAction, WorkspaceRole, parse_action, parse_role},
    enterprise::policy,
};
use crate::http::error::AppError;
use crate::http::middleware::jwt::AuthContext;

use db::{SimulationAccess, SimulationSubject};
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
    let action = parse_action(&input.action)?;
    let target_role = input
        .resource
        .as_ref()
        .and_then(|resource| resource.target_role.as_deref())
        .map(parse_role)
        .transpose()?;
    let resource = input
        .resource
        .unwrap_or_default()
        .into_resource(target_role);
    let subject = db::resolve_subject(db, tenant_id, input.workspace_id, input.subject).await?;
    let access =
        db::load_simulated_access(db, tenant_id, input.workspace_id, subject.principal_id).await?;
    let decision = build_decision(input.workspace_id, action, resource, subject, access);

    Ok(EnterprisePolicySimulationResponse { decision })
}

async fn ensure_policy_manager(
    db: &PgPool,
    auth: &AuthContext,
    tenant_id: Uuid,
) -> Result<(), AppError> {
    crate::domains::authz::ensure_tenant_management_access(db, auth, tenant_id).await?;
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

fn build_decision(
    workspace_id: Uuid,
    action: WorkspaceAction,
    mut resource: ResourceContext,
    subject: SimulationSubject,
    access: Option<SimulationAccess>,
) -> EnterprisePolicySimulationDecision {
    let Some(access) = access else {
        return subject.decision(
            false,
            "workspace_access_denied",
            action,
            None,
            false,
            workspace_id,
        );
    };

    if !resource.member_share_links_enabled {
        resource.member_share_links_enabled = access.member_share_links_enabled;
    }

    if subject.email_verified_at.is_none() {
        return subject.decision(
            false,
            "email_not_verified",
            action,
            Some(access.role),
            false,
            workspace_id,
        );
    }

    if !is_allowed(access.role, action, resource) {
        return subject.decision(
            false,
            "permission_denied",
            action,
            Some(access.role),
            false,
            workspace_id,
        );
    }

    let requires_step_up = subject.subject_type == "user" && action_requires_step_up(action);
    subject.decision(
        true,
        "allowed",
        action,
        Some(access.role),
        requires_step_up,
        workspace_id,
    )
}

impl SimulationSubject {
    fn decision(
        self,
        allowed: bool,
        reason: &str,
        action: WorkspaceAction,
        role: Option<WorkspaceRole>,
        requires_step_up: bool,
        workspace_id: Uuid,
    ) -> EnterprisePolicySimulationDecision {
        EnterprisePolicySimulationDecision {
            allowed,
            reason: reason.to_string(),
            action: action.as_str().to_string(),
            workspace_id,
            subject_type: self.subject_type,
            subject_id: self.subject_id,
            subject_label: self.subject_label,
            role: role.map(|role| role.to_string()),
            requires_step_up,
        }
    }
}

impl types::EnterprisePolicySimulationResourceInput {
    fn into_resource(self, target_role: Option<WorkspaceRole>) -> ResourceContext {
        ResourceContext {
            owns_resource: self.owns_resource,
            member_share_links_enabled: self.member_share_links_enabled,
            target_role,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Utc};

    #[test]
    fn denied_when_subject_has_no_workspace_membership() {
        let decision = build_decision(
            Uuid::nil(),
            WorkspaceAction::ViewFiles,
            ResourceContext::default(),
            subject(Some(Utc::now())),
            None,
        );

        assert!(!decision.allowed);
        assert_eq!(decision.reason, "workspace_access_denied");
        assert_eq!(decision.role, None);
    }

    #[test]
    fn user_allowed_decision_reports_step_up_requirement() {
        let decision = build_decision(
            Uuid::nil(),
            WorkspaceAction::DeleteWorkspace,
            ResourceContext::default(),
            subject(Some(Utc::now())),
            Some(SimulationAccess {
                role: WorkspaceRole::Owner,
                member_share_links_enabled: false,
            }),
        );

        assert!(decision.allowed);
        assert_eq!(decision.reason, "allowed");
        assert!(decision.requires_step_up);
    }

    #[test]
    fn unverified_user_is_denied_before_role_policy() {
        let decision = build_decision(
            Uuid::nil(),
            WorkspaceAction::ViewFiles,
            ResourceContext::default(),
            subject(None),
            Some(SimulationAccess {
                role: WorkspaceRole::Owner,
                member_share_links_enabled: false,
            }),
        );

        assert!(!decision.allowed);
        assert_eq!(decision.reason, "email_not_verified");
    }

    fn subject(email_verified_at: Option<DateTime<Utc>>) -> SimulationSubject {
        SimulationSubject {
            principal_id: Uuid::nil(),
            subject_type: "user".to_string(),
            subject_id: Uuid::nil().to_string(),
            subject_label: "user@example.test".to_string(),
            email_verified_at,
        }
    }
}
