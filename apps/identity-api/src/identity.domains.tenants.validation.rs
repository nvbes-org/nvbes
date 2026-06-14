use crate::http::error::AppError;
use nvbes_core::authz::{IdentityRole, parse_identity_role};

pub fn parse_identity_role_input(role: &str) -> Result<IdentityRole, AppError> {
    parse_identity_role(role.trim()).ok_or_else(|| {
        AppError::bad_request(
            "validation_failed",
            format!("Unsupported identity role: {role}."),
        )
    })
}

pub fn role_as_db(role: IdentityRole) -> &'static str {
    match role {
        IdentityRole::Owner => "owner",
        IdentityRole::Admin => "admin",
        IdentityRole::SecurityAdmin => "security_admin",
        IdentityRole::BillingAdmin => "billing_admin",
        IdentityRole::Member => "member",
    }
}

pub fn require_name(value: &str) -> Result<String, AppError> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.chars().count() > 120 {
        return Err(AppError::bad_request(
            "validation_failed",
            "Name must be between 1 and 120 characters.",
        ));
    }
    Ok(trimmed.to_string())
}

pub fn require_slug(value: &str) -> Result<String, AppError> {
    let slug = value.trim().to_ascii_lowercase();
    let valid = slug.len() <= 80
        && slug
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
        && !slug.starts_with('-')
        && !slug.ends_with('-');
    if slug.is_empty() || !valid {
        return Err(AppError::bad_request(
            "validation_failed",
            "Slug must contain lowercase letters, digits, or hyphens.",
        ));
    }
    Ok(slug)
}

pub fn require_token_value(value: &str, field: &str) -> Result<String, AppError> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > 64
        || !value
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_' || ch == '-')
    {
        return Err(AppError::bad_request(
            "validation_failed",
            format!("{field} is invalid."),
        ));
    }
    Ok(value.to_string())
}

pub fn require_status(value: &str) -> Result<String, AppError> {
    match value.trim() {
        "active" | "suspended" | "deleted" => Ok(value.trim().to_string()),
        _ => Err(AppError::bad_request(
            "validation_failed",
            "Organization status is invalid.",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::require_slug;

    #[test]
    fn require_slug_rejects_unsafe_values() {
        assert!(require_slug("example-team").is_ok());
        assert!(require_slug("../team").is_err());
        assert!(require_slug("-team").is_err());
    }
}
