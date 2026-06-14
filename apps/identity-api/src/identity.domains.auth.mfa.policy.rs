use sqlx::Row;
use uuid::Uuid;

use crate::http::error::AppError;

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
          COALESCE(t.mfa_policy, 'optional') AS tenant_policy,
          workspace_context.workspace_id,
          workspace_context.role,
          COALESCE(workspace_context.mfa_policy, 'optional') AS workspace_policy
        FROM principals p
        INNER JOIN tenants t ON t.id = p.tenant_id
        LEFT JOIN LATERAL (
          SELECT wm.workspace_id, wm.role::text AS role, wp.mfa_policy
          FROM workspace_memberships wm
          INNER JOIN workspace_policies wp ON wp.workspace_id = wm.workspace_id
          WHERE wm.principal_id = p.id
            AND wm.status = 'active'
          ORDER BY
            CASE wm.role WHEN 'owner' THEN 0 WHEN 'admin' THEN 1 WHEN 'member' THEN 2 ELSE 3 END,
            wm.created_at ASC
          LIMIT 1
        ) workspace_context ON TRUE
        WHERE p.id = $1
        LIMIT 1
        "#,
    )
    .bind(principal_id)
    .fetch_one(db)
    .await?;

    Ok(MfaPolicyContext {
        principal_id,
        tenant_id: row.get("tenant_id"),
        workspace_id: row.get("workspace_id"),
        workspace_role: row.get("role"),
        tenant_policy: parse_mfa_policy(row.get::<String, _>("tenant_policy").as_str()),
        workspace_policy: parse_mfa_policy(row.get::<String, _>("workspace_policy").as_str()),
        has_active_factor,
    })
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
