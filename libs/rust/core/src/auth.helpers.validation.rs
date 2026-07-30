use crate::http::error::AppError;

pub const MIN_PASSWORD_LENGTH: usize = 8;
pub const MAX_PASSWORD_LENGTH: usize = 128;

pub fn normalize_email(email: &str) -> String {
    email.trim().to_lowercase()
}

pub fn slugify(value: &str) -> String {
    let mut slug = String::with_capacity(value.len());
    let mut previous_dash = false;

    for ch in value.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            previous_dash = false;
        } else if !previous_dash {
            slug.push('-');
            previous_dash = true;
        }
    }

    while slug.starts_with('-') {
        slug.remove(0);
    }
    while slug.ends_with('-') {
        slug.pop();
    }

    if slug.is_empty() {
        "tenant".to_string()
    } else {
        slug
    }
}

pub fn validate_email(email: &str) -> Result<(), AppError> {
    if email.len() < 3 || !email.contains('@') || email.starts_with('@') || email.ends_with('@') {
        return Err(AppError::bad_request(
            "validation_failed",
            "Email address is invalid.",
        ));
    }

    Ok(())
}

pub fn validate_password(password: &str) -> Result<(), AppError> {
    let character_count = password.chars().count();
    if character_count < MIN_PASSWORD_LENGTH {
        return Err(AppError::bad_request(
            "validation_failed",
            format!("Password must be at least {MIN_PASSWORD_LENGTH} characters long."),
        ));
    }
    if character_count > MAX_PASSWORD_LENGTH {
        return Err(AppError::bad_request(
            "validation_failed",
            format!("Password must not exceed {MAX_PASSWORD_LENGTH} characters."),
        ));
    }

    let entropy = zxcvbn::zxcvbn(password, &[]);
    if (entropy.score() as u8) < 3 {
        return Err(AppError::bad_request(
            "weak_password",
            "Password is too weak. Please choose a stronger password.",
        ));
    }

    Ok(())
}

pub fn require_non_empty(field: &'static str, value: &str) -> Result<String, AppError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AppError::bad_request(
            "validation_failed",
            format!("{field} is required."),
        ));
    }

    Ok(trimmed.to_string())
}
