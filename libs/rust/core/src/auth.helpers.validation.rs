use crate::http::error::AppError;

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
    if password.len() < 8 {
        return Err(AppError::bad_request(
            "validation_failed",
            "Password must be at least 8 characters long.",
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
