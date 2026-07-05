use crate::database::Database;
use crate::domains::authz::{AdminScope, resolve_admin_scope};
use crate::domains::enterprise::{db, policy};
use crate::http::error::AppError;
use crate::http::middleware::jwt::AuthContext;
use uuid::Uuid;

use super::super::types::*;
use super::access::{ensure_member_manager, fetch_user_view};

const MAX_REASON_LEN: usize = 1_000;
const MAX_PROCEDURE_REF_LEN: usize = 255;

pub async fn activate_break_glass_account(
    db: &Database,
    redis: &nvbes_redis::RedisPool,
    auth: &AuthContext,
    tenant_id: Uuid,
    user_id: Uuid,
    input: EnterpriseBreakGlassInput,
) -> Result<EnterpriseAccessUpdateResponse, AppError> {
    let scope = resolve_admin_scope(db, auth, tenant_id, auth.organization_id).await?;
    if !matches!(scope, AdminScope::Tenant) {
        return Err(AppError::forbidden(
            "tenant_scope_required",
            "This action requires tenant-wide administrative privileges.",
        ));
    }
    let actor_access = ensure_member_manager(db, redis, auth, tenant_id, scope).await?;
    ensure_owner(&actor_access)?;
    let reason = validate_text("reason", input.reason, MAX_REASON_LEN)?;
    let procedure_reference = validate_text(
        "procedure_reference",
        input.procedure_reference,
        MAX_PROCEDURE_REF_LEN,
    )?;

    let mut tx = db.begin().await?;
    let target_role = db::target_role(&mut tx, tenant_id, user_id, AdminScope::Tenant)
        .await?
        .ok_or_else(|| {
            AppError::not_found("enterprise_user_not_found", "Tenant member not found.")
        })?;
    ensure_privileged_target(&target_role)?;
    db::upsert_break_glass_account(
        &mut tx,
        tenant_id,
        user_id,
        auth.user_id,
        &procedure_reference,
        &reason,
    )
    .await?;
    db::insert_audit(
        &mut tx,
        tenant_id,
        auth.user_id,
        "enterprise.break_glass.activated",
        "principal",
        Some(user_id),
        serde_json::json!({
            "target_role": target_role,
            "reason": reason,
            "procedure_reference": procedure_reference
        }),
    )
    .await?;
    tx.commit().await?;
    Ok(EnterpriseAccessUpdateResponse {
        user: fetch_user_view(db, tenant_id, user_id, scope).await?,
    })
}

pub async fn revoke_break_glass_account(
    db: &Database,
    redis: &nvbes_redis::RedisPool,
    auth: &AuthContext,
    tenant_id: Uuid,
    user_id: Uuid,
    input: EnterpriseAuditReasonInput,
) -> Result<EnterpriseAccessUpdateResponse, AppError> {
    let scope = resolve_admin_scope(db, auth, tenant_id, auth.organization_id).await?;
    if !matches!(scope, AdminScope::Tenant) {
        return Err(AppError::forbidden(
            "tenant_scope_required",
            "This action requires tenant-wide administrative privileges.",
        ));
    }
    let actor_access = ensure_member_manager(db, redis, auth, tenant_id, scope).await?;
    ensure_owner(&actor_access)?;
    let reason = validate_text("reason", input.reason, MAX_REASON_LEN)?;

    let mut tx = db.begin().await?;
    let changed =
        db::revoke_break_glass_account(&mut tx, tenant_id, user_id, auth.user_id, &reason).await?;
    if changed == 0 {
        return Err(AppError::not_found(
            "break_glass_account_not_found",
            "Active break-glass account not found.",
        ));
    }
    db::insert_audit(
        &mut tx,
        tenant_id,
        auth.user_id,
        "enterprise.break_glass.revoked",
        "principal",
        Some(user_id),
        serde_json::json!({"reason": reason}),
    )
    .await?;
    tx.commit().await?;
    Ok(EnterpriseAccessUpdateResponse {
        user: fetch_user_view(db, tenant_id, user_id, scope).await?,
    })
}

pub(super) fn validate_break_glass_procedure(
    reason: Option<String>,
    procedure_reference: Option<String>,
) -> Result<(String, String), AppError> {
    Ok((
        validate_text("reason", reason.unwrap_or_default(), MAX_REASON_LEN)?,
        validate_text(
            "procedure_reference",
            procedure_reference.unwrap_or_default(),
            MAX_PROCEDURE_REF_LEN,
        )?,
    ))
}

fn ensure_owner(actor_access: &super::access::ActorAccess) -> Result<(), AppError> {
    if policy::role_as_db(&actor_access.role) != "owner" {
        return Err(AppError::forbidden(
            "owner_required",
            "Only owners can manage break-glass accounts.",
        ));
    }
    Ok(())
}

fn ensure_privileged_target(target_role: &str) -> Result<(), AppError> {
    if target_role != "owner" && target_role != "admin" {
        return Err(AppError::bad_request(
            "break_glass_privileged_role_required",
            "Break-glass accounts must hold owner or admin access.",
        ));
    }
    Ok(())
}

fn validate_text(field: &str, value: String, max_len: usize) -> Result<String, AppError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AppError::bad_request(
            "validation_failed",
            format!("{field} is required."),
        ));
    }
    if trimmed.len() > max_len {
        return Err(AppError::bad_request(
            "validation_failed",
            format!("{field} must be at most {max_len} characters."),
        ));
    }
    Ok(trimmed.to_string())
}

#[cfg(test)]
mod tests {
    use super::validate_break_glass_procedure;

    #[test]
    fn validates_required_break_glass_procedure_fields() {
        let err = validate_break_glass_procedure(Some("incident".to_string()), None)
            .expect_err("missing procedure should fail");
        assert_eq!(err.code, "validation_failed");
    }

    #[test]
    fn trims_break_glass_procedure_fields() {
        let (reason, procedure) = validate_break_glass_procedure(
            Some("  incident response  ".to_string()),
            Some("  IR-42  ".to_string()),
        )
        .expect("procedure should be valid");
        assert_eq!(reason, "incident response");
        assert_eq!(procedure, "IR-42");
    }
}
