use axum::http::HeaderMap;

use crate::error::AppError;

const ROLE_HEADER: &str = "x-nvbes-backoffice-role";
const IDEMPOTENCY_KEY_HEADER: &str = "idempotency-key";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BackofficePermission {
    AccessMutate,
    BillingMutate,
    CommunicationsMutate,
    DeveloperMutate,
    EntitlementsMutate,
    GovernanceMutate,
    SecurityMutate,
    TenantLifecycle,
    UsageMutate,
    UserLifecycle,
    WorkspaceLifecycle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BackofficeRole {
    FinanceAdmin,
    PlatformAdmin,
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
    Err(AppError::forbidden(
        "backoffice_permission_denied",
        "Back-office operator role is not allowed to execute this action.",
    ))
}

pub(crate) fn require_confirmation(actual: &str, expected: &str) -> Result<(), AppError> {
    if actual.trim() == expected {
        return Ok(());
    }
    Err(AppError::bad_request(
        "backoffice_confirmation_required",
        "Back-office action requires the exact confirmation code.",
    ))
}

pub(crate) fn require_idempotency_key(headers: &HeaderMap) -> Result<&str, AppError> {
    let value = headers
        .get(IDEMPOTENCY_KEY_HEADER)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| {
            AppError::bad_request(
                "idempotency_key_required",
                "Back-office mutations require Idempotency-Key.",
            )
        })?;
    nvbes_core::idempotency::validate_key(value).map_err(|message| {
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
            AppError::unauthorized(
                "backoffice_role_required",
                "Back-office requests require x-nvbes-backoffice-role.",
            )
        })?;
    parse_role(value)
}

fn parse_role(value: &str) -> Result<BackofficeRole, AppError> {
    match value.trim() {
        "finance_admin" => Ok(BackofficeRole::FinanceAdmin),
        "platform_admin" => Ok(BackofficeRole::PlatformAdmin),
        "security_admin" => Ok(BackofficeRole::SecurityAdmin),
        "support_agent" => Ok(BackofficeRole::SupportAgent),
        "viewer" => Ok(BackofficeRole::Viewer),
        _ => Err(AppError::bad_request(
            "invalid_backoffice_role",
            "Back-office role is not recognized.",
        )),
    }
}

fn role_allows(role: BackofficeRole, permission: BackofficePermission) -> bool {
    match role {
        BackofficeRole::PlatformAdmin => true,
        BackofficeRole::FinanceAdmin => matches!(
            permission,
            BackofficePermission::BillingMutate
                | BackofficePermission::DeveloperMutate
                | BackofficePermission::EntitlementsMutate
                | BackofficePermission::UsageMutate
        ),
        BackofficeRole::SecurityAdmin => matches!(
            permission,
            BackofficePermission::AccessMutate
                | BackofficePermission::GovernanceMutate
                | BackofficePermission::SecurityMutate
                | BackofficePermission::UserLifecycle
        ),
        BackofficeRole::SupportAgent => matches!(
            permission,
            BackofficePermission::CommunicationsMutate
                | BackofficePermission::TenantLifecycle
                | BackofficePermission::UserLifecycle
                | BackofficePermission::WorkspaceLifecycle
        ),
        BackofficeRole::Viewer => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_admin_can_run_every_tracked_permission() {
        assert!(role_allows(
            BackofficeRole::PlatformAdmin,
            BackofficePermission::TenantLifecycle
        ));
        assert!(role_allows(
            BackofficeRole::PlatformAdmin,
            BackofficePermission::AccessMutate
        ));
        assert!(role_allows(
            BackofficeRole::PlatformAdmin,
            BackofficePermission::SecurityMutate
        ));
        assert!(role_allows(
            BackofficeRole::PlatformAdmin,
            BackofficePermission::EntitlementsMutate
        ));
        assert!(role_allows(
            BackofficeRole::PlatformAdmin,
            BackofficePermission::UsageMutate
        ));
        assert!(role_allows(
            BackofficeRole::PlatformAdmin,
            BackofficePermission::DeveloperMutate
        ));
    }

    #[test]
    fn viewer_cannot_mutate() {
        assert!(!role_allows(
            BackofficeRole::Viewer,
            BackofficePermission::TenantLifecycle
        ));
    }

    #[test]
    fn security_admin_can_mutate_security_and_governance() {
        assert!(role_allows(
            BackofficeRole::SecurityAdmin,
            BackofficePermission::SecurityMutate
        ));
        assert!(role_allows(
            BackofficeRole::SecurityAdmin,
            BackofficePermission::GovernanceMutate
        ));
        assert!(!role_allows(
            BackofficeRole::SecurityAdmin,
            BackofficePermission::TenantLifecycle
        ));
    }

    #[test]
    fn finance_admin_can_only_mutate_billing() {
        assert!(role_allows(
            BackofficeRole::FinanceAdmin,
            BackofficePermission::BillingMutate
        ));
        assert!(role_allows(
            BackofficeRole::FinanceAdmin,
            BackofficePermission::EntitlementsMutate
        ));
        assert!(role_allows(
            BackofficeRole::FinanceAdmin,
            BackofficePermission::UsageMutate
        ));
        assert!(role_allows(
            BackofficeRole::FinanceAdmin,
            BackofficePermission::DeveloperMutate
        ));
        assert!(!role_allows(
            BackofficeRole::FinanceAdmin,
            BackofficePermission::SecurityMutate
        ));
    }

    #[test]
    fn confirmation_must_match_exactly_after_trim() {
        assert!(require_confirmation("SUSPEND TENANT", "SUSPEND TENANT").is_ok());
        assert!(require_confirmation(" SUSPEND TENANT ", "SUSPEND TENANT").is_ok());
        assert!(require_confirmation("suspend tenant", "SUSPEND TENANT").is_err());
    }

    #[test]
    fn idempotency_key_is_required_and_validated() {
        let headers = HeaderMap::new();
        assert!(require_idempotency_key(&headers).is_err());

        let mut headers = HeaderMap::new();
        headers.insert(IDEMPOTENCY_KEY_HEADER, " key-123 ".parse().unwrap());
        assert_eq!(require_idempotency_key(&headers).unwrap(), "key-123");
    }
}
