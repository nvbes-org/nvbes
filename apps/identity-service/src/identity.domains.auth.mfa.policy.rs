use sqlx::Row;
use uuid::Uuid;

use crate::{
    cloud_boundary::workspace_port::{self, CloudWorkspaceMemberSummary},
    http::error::AppError,
};

const PRIVILEGED_ROLES: &[&str] = &["owner", "admin", "security_admin", "billing_admin"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MfaPolicy {
    Optional,
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
    pub account_policy: MfaPolicy,
    pub has_active_factor: bool,
}

pub fn parse_mfa_policy(value: &str) -> MfaPolicy {
    match value {
        "required_all" => MfaPolicy::RequiredForEveryone,
        _ => MfaPolicy::Optional,
    }
}

pub fn mfa_policy_as_str(policy: MfaPolicy) -> &'static str {
    match policy {
        MfaPolicy::Optional => "optional",
        MfaPolicy::RequiredForEveryone => "required_all",
    }
}

pub fn evaluate_mfa_policy(context: &MfaPolicyContext) -> MfaPolicyDecision {
    if context.has_active_factor {
        return MfaPolicyDecision::Challenge;
    }

    if context.account_policy == MfaPolicy::RequiredForEveryone {
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
    Ok(MfaPolicyContext {
        principal_id,
        tenant_id: row.get("tenant_id"),
        account_policy: parse_mfa_policy(row.get::<String, _>("tenant_policy").as_str()),
        has_active_factor,
    })
}

pub async fn principal_has_privileged_role(
    db: &sqlx::PgPool,
    principal_id: Uuid,
) -> Result<bool, AppError> {
    let account_roles = sqlx::query_scalar::<_, String>(
        r#"
        SELECT role::text
        FROM tenant_memberships
        WHERE principal_id = $1
          AND status = 'active'
        UNION ALL
        SELECT role::text
        FROM organization_memberships
        WHERE principal_id = $1
          AND status = 'active'
        "#,
    )
    .bind(principal_id)
    .fetch_all(db)
    .await?;

    if account_roles.iter().any(|role| is_privileged_role(role)) {
        return Ok(true);
    }

    principal_has_privileged_workspace_role(principal_id).await
}

async fn principal_has_privileged_workspace_role(principal_id: Uuid) -> Result<bool, AppError> {
    for workspace in workspace_port::list_workspaces(None, principal_id).await? {
        let members = workspace_port::list_workspace_members(
            Some(workspace.tenant_id),
            workspace.workspace_id,
            principal_id,
        )
        .await?;
        if members_include_privileged_principal(&members, principal_id) {
            return Ok(true);
        }
    }

    Ok(false)
}

fn members_include_privileged_principal(
    members: &[CloudWorkspaceMemberSummary],
    principal_id: Uuid,
) -> bool {
    members.iter().any(|member| {
        member.principal_id == principal_id && member.active && is_privileged_role(&member.role)
    })
}

fn is_privileged_role(role: &str) -> bool {
    PRIVILEGED_ROLES.contains(&role)
}

pub fn methods_for_privileged_principal(
    methods: Vec<String>,
    privileged: bool,
) -> Result<Vec<String>, AppError> {
    if !privileged {
        return Ok(methods);
    }

    if methods.iter().any(|method| method == "webauthn") {
        return Ok(vec!["webauthn".to_string()]);
    }

    Err(AppError::forbidden(
        "privileged_passkey_required",
        "Privileged accounts must enroll a passkey or hardware security key before signing in.",
    ))
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;

    fn context(account_policy: MfaPolicy, has_active_factor: bool) -> MfaPolicyContext {
        MfaPolicyContext {
            principal_id: Uuid::nil(),
            tenant_id: Uuid::nil(),
            account_policy,
            has_active_factor,
        }
    }

    fn workspace_member(
        principal_id: Uuid,
        role: &str,
        active: bool,
    ) -> CloudWorkspaceMemberSummary {
        CloudWorkspaceMemberSummary {
            principal_id,
            role: role.to_string(),
            status: if active { "active" } else { "removed" }.to_string(),
            active,
            joined_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn active_factor_always_requires_challenge() {
        let decision = evaluate_mfa_policy(&context(MfaPolicy::Optional, true));

        assert_eq!(decision, MfaPolicyDecision::Challenge);
    }

    #[test]
    fn optional_policy_allows_login_without_factor() {
        let decision = evaluate_mfa_policy(&context(MfaPolicy::Optional, false));

        assert_eq!(decision, MfaPolicyDecision::Optional);
    }

    #[test]
    fn required_policy_requires_enrollment_without_factor() {
        let decision = evaluate_mfa_policy(&context(MfaPolicy::RequiredForEveryone, false));

        assert_eq!(decision, MfaPolicyDecision::EnrollmentRequired);
    }

    #[test]
    fn privileged_principals_use_webauthn_when_available() {
        let methods = methods_for_privileged_principal(
            vec![
                "totp".to_string(),
                "webauthn".to_string(),
                "recovery".to_string(),
            ],
            true,
        )
        .expect("WebAuthn should satisfy the privileged policy");

        assert_eq!(methods, vec!["webauthn"]);
    }

    #[test]
    fn privileged_principals_cannot_fall_back_to_totp() {
        let error = methods_for_privileged_principal(vec!["totp".to_string()], true)
            .expect_err("TOTP must not satisfy the privileged policy");

        assert_eq!(error.code, "privileged_passkey_required");
    }

    #[test]
    fn every_privileged_workspace_role_is_recognized() {
        let principal_id = Uuid::new_v4();

        for role in PRIVILEGED_ROLES {
            assert!(members_include_privileged_principal(
                &[workspace_member(principal_id, role, true)],
                principal_id,
            ));
        }
    }

    #[test]
    fn inactive_or_unrelated_workspace_members_do_not_grant_privilege() {
        let principal_id = Uuid::new_v4();
        let members = vec![
            workspace_member(principal_id, "owner", false),
            workspace_member(Uuid::new_v4(), "admin", true),
            workspace_member(principal_id, "member", true),
        ];

        assert!(!members_include_privileged_principal(
            &members,
            principal_id
        ));
    }
}

#[cfg(test)]
#[path = "identity.domains.auth.mfa.policy.boundary.tests.rs"]
mod boundary_tests;
