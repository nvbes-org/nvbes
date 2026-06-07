use crate::http::error::AppError;
use nvbes_tenancy::workspace::WorkspaceInputError;

pub fn validate_workspace_name(name: &str) -> Result<String, AppError> {
    nvbes_tenancy::workspace::validate_workspace_name(name).map_err(map_workspace_input_error)
}

pub fn parse_workspace_type(value: Option<&str>) -> Result<&'static str, AppError> {
    nvbes_tenancy::workspace::parse_workspace_type(value).map_err(map_workspace_input_error)
}

fn map_workspace_input_error(error: WorkspaceInputError) -> AppError {
    AppError::bad_request("validation_failed", error.to_string())
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
