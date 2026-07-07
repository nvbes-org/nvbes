use crate::error::AppError;

pub(crate) fn validate_operator_role(role: &str) -> Result<&str, AppError> {
    match role.trim() {
        "compliance_admin" | "developer_admin" | "finance_admin" | "operations_admin"
        | "platform_admin" | "product_admin" | "security_admin" | "support_agent" | "viewer" => {
            Ok(role.trim())
        }
        _ => Err(AppError::bad_request(
            "invalid_operator_role",
            "Back-office operator role is not recognized.",
        )),
    }
}

pub(crate) fn validate_operator_grant_reason(reason: &str) -> Result<(), AppError> {
    if reason.trim().len() < 12 {
        return Err(AppError::bad_request(
            "audit_reason_required",
            "Operator grant actions require a detailed audit reason.",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_operator_roles() {
        assert!(validate_operator_role("platform_admin").is_ok());
        assert!(validate_operator_role("owner").is_err());
    }

    #[test]
    fn operator_grant_reason_must_be_detailed() {
        assert!(validate_operator_grant_reason("short").is_err());
        assert!(validate_operator_grant_reason("ticket IAM-123 approved").is_ok());
    }
}
