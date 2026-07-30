use crate::http::error::AppError;

pub const MAX_USERNAME_LENGTH: usize = 100;

pub fn normalize_username(value: &str) -> Result<String, AppError> {
    let username = value.trim();

    if username.is_empty() {
        return Err(AppError::bad_request(
            "username_required",
            "Username is required.",
        ));
    }

    if username.chars().count() > MAX_USERNAME_LENGTH {
        return Err(AppError::bad_request(
            "username_too_long",
            format!("Username must be {MAX_USERNAME_LENGTH} characters or fewer."),
        ));
    }

    Ok(username.to_string())
}

#[cfg(test)]
mod tests {
    use super::{MAX_USERNAME_LENGTH, normalize_username};

    #[test]
    fn username_is_trimmed_and_limited_to_one_hundred_characters() {
        assert_eq!(normalize_username("  shayn  ").unwrap(), "shayn");
        assert!(normalize_username(&"a".repeat(MAX_USERNAME_LENGTH + 1)).is_err());
    }

    #[test]
    fn username_cannot_be_blank() {
        assert!(normalize_username("   ").is_err());
    }
}
