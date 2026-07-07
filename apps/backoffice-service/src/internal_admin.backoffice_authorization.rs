use axum::http::HeaderMap;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;
use crate::observability::{record_guard_rejection, record_permission_denied};

const ACTOR_HEADER: &str = "x-nvbes-actor-principal-id";
const ROLE_HEADER: &str = "x-nvbes-backoffice-role";
const IDEMPOTENCY_KEY_HEADER: &str = "idempotency-key";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BackofficePermission {
    AccessMutate,
    BillingMutate,
    BillingPlatformMutate,
    CommunicationsMutate,
    ComplianceMutate,
    DeveloperMutate,
    EntitlementsMutate,
    GovernanceMutate,
    OperationsMutate,
    RegionMutate,
    RevenueMutate,
    RiskMutate,
    SecurityMutate,
    TenantLifecycle,
    UsageMutate,
    UserLifecycle,
    WorkspaceLifecycle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BackofficeRole {
    ComplianceAdmin,
    DeveloperAdmin,
    FinanceAdmin,
    OperationsAdmin,
    PlatformAdmin,
    ProductAdmin,
    SecurityAdmin,
    SupportAgent,
    Viewer,
}

pub(crate) fn require_permission(
    headers: &HeaderMap,
    permission: BackofficePermission,
) -> Result<(), AppError> {
    let role = backoffice_role(headers)?;
    if role_allows(role, permission) {
        return Ok(());
    }
    record_permission_denied(permission, role.metric_name());
    Err(AppError::forbidden(
        "backoffice_permission_denied",
        "Back-office operator role is not allowed to execute this action.",
    ))
}

pub(crate) async fn require_operator_role_grant(
    db: &PgPool,
    headers: &HeaderMap,
) -> Result<(), AppError> {
    let actor_id = backoffice_actor(headers)?;
    let role = backoffice_role(headers)?;
    let role_name = role.metric_name();
    let has_grant = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
          SELECT 1
          FROM internal_admin_operator_grants
          WHERE principal_id = $1 AND role = $2 AND status = 'active'
        )
        "#,
    )
    .bind(actor_id)
    .bind(role_name)
    .fetch_one(db)
    .await?;

    if has_grant {
        return Ok(());
    }
    record_guard_rejection("rbac", "missing_operator_grant");
    Err(AppError::forbidden(
        "backoffice_operator_grant_required",
        "Back-office actor is not granted the requested operator role.",
    ))
}

pub(crate) fn require_operator_permission_headers(
    headers: &HeaderMap,
    permission: BackofficePermission,
) -> Result<(), AppError> {
    require_idempotency_key(headers)?;
    require_permission(headers, permission)
}

pub(crate) fn require_confirmation(actual: &str, expected: &str) -> Result<(), AppError> {
    if actual.trim() == expected {
        return Ok(());
    }
    record_guard_rejection("strong_confirmation", "mismatch");
    Err(AppError::bad_request(
        "backoffice_confirmation_required",
        "Back-office action requires the exact confirmation code.",
    ))
}

pub(crate) fn require_strong_confirmation(
    actual: &str,
    expected_action: &str,
    target_id: Uuid,
) -> Result<(), AppError> {
    require_confirmation(
        actual,
        &strong_confirmation_code(expected_action, target_id),
    )
}

pub(crate) fn require_strong_confirmation_for_value(
    actual: &str,
    expected_action: &str,
    target_id: &str,
) -> Result<(), AppError> {
    require_confirmation(
        actual,
        &strong_confirmation_code_for_value(expected_action, target_id),
    )
}

pub(crate) fn strong_confirmation_code(expected_action: &str, target_id: Uuid) -> String {
    strong_confirmation_code_for_value(expected_action, &target_id.simple().to_string())
}

pub(crate) fn strong_confirmation_code_for_value(expected_action: &str, target_id: &str) -> String {
    let fingerprint: String = target_id
        .to_string()
        .chars()
        .filter(|character| *character != '-')
        .take(8)
        .collect::<String>()
        .to_ascii_uppercase();
    format!("{expected_action} {fingerprint}")
}

pub(crate) fn require_idempotency_key(headers: &HeaderMap) -> Result<&str, AppError> {
    let value = headers
        .get(IDEMPOTENCY_KEY_HEADER)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| {
            record_guard_rejection("idempotency", "missing_key");
            AppError::bad_request(
                "idempotency_key_required",
                "Back-office mutations require Idempotency-Key.",
            )
        })?;
    nvbes_core::idempotency::validate_key(value).map_err(|message| {
        record_guard_rejection("idempotency", "invalid_key");
        AppError::bad_request(
            "invalid_idempotency_key",
            format!("Invalid Idempotency-Key: {message}"),
        )
    })
}

fn backoffice_role(headers: &HeaderMap) -> Result<BackofficeRole, AppError> {
    let value = headers
        .get(ROLE_HEADER)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| {
            record_guard_rejection("rbac", "missing_role");
            AppError::unauthorized(
                "backoffice_role_required",
                "Back-office requests require x-nvbes-backoffice-role.",
            )
        })?;
    parse_role(value)
}

fn backoffice_actor(headers: &HeaderMap) -> Result<Uuid, AppError> {
    let value = headers
        .get(ACTOR_HEADER)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| {
            record_guard_rejection("rbac", "missing_actor");
            AppError::unauthorized(
                "backoffice_actor_required",
                "Back-office requests require x-nvbes-actor-principal-id.",
            )
        })?;
    Uuid::parse_str(value).map_err(|_| {
        record_guard_rejection("rbac", "invalid_actor");
        AppError::bad_request(
            "invalid_backoffice_actor",
            "Back-office actor principal ID must be a UUID.",
        )
    })
}

fn parse_role(value: &str) -> Result<BackofficeRole, AppError> {
    match value.trim() {
        "compliance_admin" => Ok(BackofficeRole::ComplianceAdmin),
        "developer_admin" => Ok(BackofficeRole::DeveloperAdmin),
        "finance_admin" => Ok(BackofficeRole::FinanceAdmin),
        "operations_admin" => Ok(BackofficeRole::OperationsAdmin),
        "platform_admin" => Ok(BackofficeRole::PlatformAdmin),
        "product_admin" => Ok(BackofficeRole::ProductAdmin),
        "security_admin" => Ok(BackofficeRole::SecurityAdmin),
        "support_agent" => Ok(BackofficeRole::SupportAgent),
        "viewer" => Ok(BackofficeRole::Viewer),
        _ => {
            record_guard_rejection("rbac", "invalid_role");
            Err(AppError::bad_request(
                "invalid_backoffice_role",
                "Back-office role is not recognized.",
            ))
        }
    }
}

impl BackofficePermission {
    pub(crate) fn metric_name(self) -> &'static str {
        match self {
            BackofficePermission::AccessMutate => "access_mutate",
            BackofficePermission::BillingMutate => "billing_mutate",
            BackofficePermission::BillingPlatformMutate => "billing_platform_mutate",
            BackofficePermission::CommunicationsMutate => "communications_mutate",
            BackofficePermission::ComplianceMutate => "compliance_mutate",
            BackofficePermission::DeveloperMutate => "developer_mutate",
            BackofficePermission::EntitlementsMutate => "entitlements_mutate",
            BackofficePermission::GovernanceMutate => "governance_mutate",
            BackofficePermission::OperationsMutate => "operations_mutate",
            BackofficePermission::RegionMutate => "region_mutate",
            BackofficePermission::RevenueMutate => "revenue_mutate",
            BackofficePermission::RiskMutate => "risk_mutate",
            BackofficePermission::SecurityMutate => "security_mutate",
            BackofficePermission::TenantLifecycle => "tenant_lifecycle",
            BackofficePermission::UsageMutate => "usage_mutate",
            BackofficePermission::UserLifecycle => "user_lifecycle",
            BackofficePermission::WorkspaceLifecycle => "workspace_lifecycle",
        }
    }
}

impl BackofficeRole {
    fn metric_name(self) -> &'static str {
        match self {
            BackofficeRole::ComplianceAdmin => "compliance_admin",
            BackofficeRole::DeveloperAdmin => "developer_admin",
            BackofficeRole::FinanceAdmin => "finance_admin",
            BackofficeRole::OperationsAdmin => "operations_admin",
            BackofficeRole::PlatformAdmin => "platform_admin",
            BackofficeRole::ProductAdmin => "product_admin",
            BackofficeRole::SecurityAdmin => "security_admin",
            BackofficeRole::SupportAgent => "support_agent",
            BackofficeRole::Viewer => "viewer",
        }
    }
}

fn role_allows(role: BackofficeRole, permission: BackofficePermission) -> bool {
    match role {
        BackofficeRole::PlatformAdmin => true,
        BackofficeRole::ComplianceAdmin => matches!(
            permission,
            BackofficePermission::ComplianceMutate | BackofficePermission::RegionMutate
        ),
        BackofficeRole::DeveloperAdmin => {
            matches!(permission, BackofficePermission::DeveloperMutate)
        }
        BackofficeRole::FinanceAdmin => matches!(
            permission,
            BackofficePermission::BillingMutate
                | BackofficePermission::BillingPlatformMutate
                | BackofficePermission::RevenueMutate
        ),
        BackofficeRole::OperationsAdmin => matches!(
            permission,
            BackofficePermission::CommunicationsMutate | BackofficePermission::OperationsMutate
        ),
        BackofficeRole::ProductAdmin => matches!(
            permission,
            BackofficePermission::EntitlementsMutate | BackofficePermission::UsageMutate
        ),
        BackofficeRole::SecurityAdmin => matches!(
            permission,
            BackofficePermission::AccessMutate
                | BackofficePermission::GovernanceMutate
                | BackofficePermission::RiskMutate
                | BackofficePermission::SecurityMutate
                | BackofficePermission::UserLifecycle
        ),
        BackofficeRole::SupportAgent => false,
        BackofficeRole::Viewer => false,
    }
}

#[cfg(test)]
#[path = "internal_admin.backoffice_authorization.tests.rs"]
mod tests;
