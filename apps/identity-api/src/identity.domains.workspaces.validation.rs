use crate::http::error::AppError;

pub fn validate_workspace_name(name: &str) -> Result<String, AppError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(AppError::bad_request(
            "validation_failed",
            "Workspace name cannot be empty.",
        ));
    }
    if trimmed.chars().count() > 120 {
        return Err(AppError::bad_request(
            "validation_failed",
            "Workspace name must be 120 characters or fewer.",
        ));
    }
    Ok(trimmed.to_owned())
}

pub fn parse_workspace_type(value: Option<&str>) -> Result<&'static str, AppError> {
    match value.unwrap_or("team").trim() {
        "personal" => Ok("personal"),
        "team" => Ok("team"),
        _ => Err(AppError::bad_request(
            "validation_failed",
            "workspace_type must be personal or team.",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_workspace_type, validate_workspace_name};

    #[test]
    fn validate_workspace_name_trims_and_rejects_blank_values() {
        assert_eq!(
            validate_workspace_name("  Acme Workspace  ").expect("workspace name should trim"),
            "Acme Workspace"
        );

        let error = validate_workspace_name("   ").expect_err("blank workspace name should fail");
        assert_eq!(error.code, "validation_failed");
    }

    #[test]
    fn parse_workspace_type_accepts_known_values() {
        assert_eq!(
            parse_workspace_type(None).expect("default should be team"),
            "team"
        );
        assert_eq!(
            parse_workspace_type(Some(" personal ")).expect("personal should be accepted"),
            "personal"
        );
    }

    #[test]
    fn parse_workspace_type_rejects_unknown_values() {
        let error = parse_workspace_type(Some("enterprise"))
            .expect_err("invalid workspace type should fail");

        assert_eq!(error.code, "validation_failed");
    }
}
