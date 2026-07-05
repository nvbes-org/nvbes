use crate::database::Database;
use crate::domains::authz::{AdminScope, resolve_admin_scope};
use crate::domains::enterprise::{db, policy};
use crate::http::error::AppError;
use crate::http::middleware::jwt::AuthContext;
use uuid::Uuid;

use super::super::types::*;

pub async fn update_session_policy(
    db: &Database,
    redis: &nvbes_redis::RedisPool,
    auth: &AuthContext,
    tenant_id: Uuid,
    fallback_ttl_hours: i64,
    input: EnterpriseSessionPolicyInput,
) -> Result<EnterprisePoliciesResponse, AppError> {
    ensure_policy_manager(db, redis, auth, tenant_id).await?;
    if !(1..=168).contains(&input.admin_session_ttl_hours) {
        return Err(AppError::bad_request(
            "validation_failed",
            "Admin session TTL must be between 1 and 168 hours.",
        ));
    }

    let mut tx = db.begin().await?;
    db::set_session_policy(&mut tx, tenant_id, input.admin_session_ttl_hours).await?;
    db::insert_audit(
        &mut tx,
        tenant_id,
        auth.user_id,
        "enterprise.policy.session_updated",
        "tenant_policy",
        Some(tenant_id),
        serde_json::json!({"admin_session_ttl_hours": input.admin_session_ttl_hours}),
    )
    .await?;
    tx.commit().await?;

    super::reads::list_policies(db, auth, tenant_id, fallback_ttl_hours).await
}

pub async fn update_mfa_policy(
    db: &Database,
    redis: &nvbes_redis::RedisPool,
    auth: &AuthContext,
    tenant_id: Uuid,
    fallback_ttl_hours: i64,
    input: EnterpriseMfaPolicyInput,
) -> Result<EnterprisePoliciesResponse, AppError> {
    ensure_policy_manager(db, redis, auth, tenant_id).await?;
    let policy_value = validate_mfa_policy(input.policy)?;

    let mut tx = db.begin().await?;
    db::set_mfa_policy(&mut tx, tenant_id, policy_value).await?;
    db::insert_audit(
        &mut tx,
        tenant_id,
        auth.user_id,
        "enterprise.policy.mfa_updated",
        "tenant",
        Some(tenant_id),
        serde_json::json!({"mfa_policy": policy_value}),
    )
    .await?;
    tx.commit().await?;

    super::reads::list_policies(db, auth, tenant_id, fallback_ttl_hours).await
}

async fn ensure_policy_manager(
    db: &Database,
    redis: &nvbes_redis::RedisPool,
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
    let access = super::access::require_actor_access(db, auth, tenant_id, scope).await?;
    if !policy::can_manage_policies(
        policy::role_as_db(&access.role),
        &policy::grant_names(&access.grants),
    ) {
        return Err(AppError::forbidden(
            "policies_grant_required",
            "Policies access is required.",
        ));
    }
    crate::domains::enterprise::admin_elevation::require_active_admin_elevation(
        redis, auth, tenant_id,
    )
    .await
    .map(|_| ())
}

fn validate_mfa_policy(policy: String) -> Result<&'static str, AppError> {
    match policy.as_str() {
        "optional" => Ok("optional"),
        "required_admins" => Ok("required_admins"),
        "required_all" => Ok("required_all"),
        _ => Err(AppError::bad_request(
            "validation_failed",
            "MFA policy must be optional, required_admins, or required_all.",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::validate_mfa_policy;

    #[test]
    fn mfa_policy_validation_accepts_supported_values() {
        assert_eq!(
            validate_mfa_policy("optional".to_string()).expect("optional should be valid"),
            "optional"
        );
        assert_eq!(
            validate_mfa_policy("required_admins".to_string())
                .expect("admin policy should be valid"),
            "required_admins"
        );
        assert_eq!(
            validate_mfa_policy("required_all".to_string()).expect("all policy should be valid"),
            "required_all"
        );
    }

    #[test]
    fn mfa_policy_validation_rejects_unknown_values() {
        let err = validate_mfa_policy("admins".to_string()).expect_err("value should be rejected");
        assert_eq!(err.code, "validation_failed");
    }
}
