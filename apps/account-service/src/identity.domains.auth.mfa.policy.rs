use sqlx::Row;
use uuid::Uuid;

use crate::{domains::cloud::workspace_port, http::error::AppError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MfaPolicy {
    Optional,
    RequiredForAdmins,
    RequiredForEveryone,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MfaPolicyDecision {
    Optional,
    Challenge,
    EnrollmentRequired,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MfaPolicyContext {
    pub principal_id: Uuid,
    pub tenant_id: Uuid,
    pub workspace_id: Option<Uuid>,
    pub workspace_role: Option<String>,
    pub tenant_policy: MfaPolicy,
    pub workspace_policy: MfaPolicy,
    pub has_active_factor: bool,
}

pub fn parse_mfa_policy(value: &str) -> MfaPolicy {
    match value {
        "required_admins" => MfaPolicy::RequiredForAdmins,
        "required_all" => MfaPolicy::RequiredForEveryone,
        _ => MfaPolicy::Optional,
    }
}

pub fn mfa_policy_as_str(policy: MfaPolicy) -> &'static str {
    match policy {
        MfaPolicy::Optional => "optional",
        MfaPolicy::RequiredForAdmins => "required_admins",
        MfaPolicy::RequiredForEveryone => "required_all",
    }
}

pub fn evaluate_mfa_policy(context: &MfaPolicyContext) -> MfaPolicyDecision {
    if context.has_active_factor {
        return MfaPolicyDecision::Challenge;
    }

    if policy_requires_mfa(context.tenant_policy, context.workspace_role.as_deref())
        || policy_requires_mfa(context.workspace_policy, context.workspace_role.as_deref())
    {
        return MfaPolicyDecision::EnrollmentRequired;
    }

    MfaPolicyDecision::Optional
}

pub async fn fetch_policy_context(
    db: &sqlx::PgPool,
    principal_id: Uuid,
    has_active_factor: bool,
) -> Result<MfaPolicyContext, AppError> {
    let row = sqlx::query(
        r#"
        SELECT
          p.tenant_id,
          COALESCE(t.mfa_policy, 'optional') AS tenant_policy
        FROM principals p
        INNER JOIN tenants t ON t.id = p.tenant_id
        WHERE p.id = $1
        LIMIT 1
        "#,
    )
    .bind(principal_id)
    .fetch_one(db)
    .await?;
    let tenant_id: Uuid = row.get("tenant_id");
    let workspace_context = mfa_workspace_context(tenant_id, principal_id).await?;

    Ok(MfaPolicyContext {
        principal_id,
        tenant_id,
        workspace_id: workspace_context
            .as_ref()
            .map(|context| context.workspace_id),
        workspace_role: workspace_context
            .as_ref()
            .map(|context| context.role.clone()),
        tenant_policy: parse_mfa_policy(row.get::<String, _>("tenant_policy").as_str()),
        workspace_policy: workspace_context
            .as_ref()
            .map(|context| parse_mfa_policy(&context.mfa_policy))
            .unwrap_or(MfaPolicy::Optional),
        has_active_factor,
    })
}

struct MfaWorkspaceContext {
    workspace_id: Uuid,
    role: String,
    mfa_policy: String,
}

async fn mfa_workspace_context(
    tenant_id: Uuid,
    principal_id: Uuid,
) -> Result<Option<MfaWorkspaceContext>, AppError> {
    let mut selected = None;
    let mut selected_rank = i32::MAX;

    for workspace in workspace_port::list_workspaces(Some(tenant_id), principal_id).await? {
        let role = workspace_port::list_workspace_members(
            Some(tenant_id),
            workspace.workspace_id,
            principal_id,
        )
        .await?
        .into_iter()
        .find(|member| member.principal_id == principal_id && member.active)
        .map(|member| member.role);
        let Some(role) = role else {
            continue;
        };
        let rank = mfa_workspace_role_rank(&role);
        if rank < selected_rank {
            selected_rank = rank;
            selected = Some(MfaWorkspaceContext {
                workspace_id: workspace.workspace_id,
                role,
                mfa_policy: workspace
                    .mfa_policy
                    .unwrap_or_else(|| "optional".to_string()),
            });
        }
    }

    Ok(selected)
}

fn mfa_workspace_role_rank(role: &str) -> i32 {
    match role {
        "owner" => 0,
        "admin" => 1,
        "member" => 2,
        _ => 3,
    }
}

fn policy_requires_mfa(policy: MfaPolicy, workspace_role: Option<&str>) -> bool {
    match policy {
        MfaPolicy::Optional => false,
        MfaPolicy::RequiredForEveryone => true,
        MfaPolicy::RequiredForAdmins => matches!(workspace_role, Some("owner" | "admin")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context(
        tenant_policy: MfaPolicy,
        workspace_policy: MfaPolicy,
        role: Option<&str>,
        has_active_factor: bool,
    ) -> MfaPolicyContext {
        MfaPolicyContext {
            principal_id: Uuid::nil(),
            tenant_id: Uuid::nil(),
            workspace_id: None,
            workspace_role: role.map(str::to_string),
            tenant_policy,
            workspace_policy,
            has_active_factor,
        }
    }

    #[test]
    fn active_factor_always_requires_challenge() {
        let decision = evaluate_mfa_policy(&context(
            MfaPolicy::Optional,
            MfaPolicy::Optional,
            Some("member"),
            true,
        ));

        assert_eq!(decision, MfaPolicyDecision::Challenge);
    }

    #[test]
    fn admin_policy_requires_admin_enrollment_without_factor() {
        let decision = evaluate_mfa_policy(&context(
            MfaPolicy::RequiredForAdmins,
            MfaPolicy::Optional,
            Some("admin"),
            false,
        ));

        assert_eq!(decision, MfaPolicyDecision::EnrollmentRequired);
    }

    #[test]
    fn admin_policy_does_not_require_member_enrollment() {
        let decision = evaluate_mfa_policy(&context(
            MfaPolicy::RequiredForAdmins,
            MfaPolicy::Optional,
            Some("member"),
            false,
        ));

        assert_eq!(decision, MfaPolicyDecision::Optional);
    }

    #[test]
    fn required_all_policy_requires_everyone() {
        let decision = evaluate_mfa_policy(&context(
            MfaPolicy::Optional,
            MfaPolicy::RequiredForEveryone,
            Some("viewer"),
            false,
        ));

        assert_eq!(decision, MfaPolicyDecision::EnrollmentRequired);
    }
}
