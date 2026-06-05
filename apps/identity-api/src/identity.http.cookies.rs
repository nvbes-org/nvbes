use crate::http::error::AppError;

pub fn auth_cookie(
    name: &str,
    value: &str,
    max_age_seconds: i64,
    secure: bool,
) -> Result<axum::http::HeaderValue, AppError> {
    let secure_flag = if secure { "; Secure" } else { "" };
    let cookie = format!(
        "{name}={value}; HttpOnly; SameSite=Strict; Path=/; Max-Age={max_age_seconds}{secure_flag}"
    );

    axum::http::HeaderValue::from_str(&cookie)
        .map_err(|e| AppError::internal("cookie_header_invalid", &format!("{}", e)))
}

pub fn csrf_cookie(
    name: &str,
    value: &str,
    max_age_seconds: i64,
    secure: bool,
) -> Result<axum::http::HeaderValue, AppError> {
    let secure_flag = if secure { "; Secure" } else { "" };
    let cookie =
        format!("{name}={value}; SameSite=Strict; Path=/; Max-Age={max_age_seconds}{secure_flag}");

    axum::http::HeaderValue::from_str(&cookie)
        .map_err(|e| AppError::internal("cookie_header_invalid", &format!("{}", e)))
}

pub fn auth_cookie_name(base: &str, secure: bool) -> String {
    if secure {
        format!("__Host-{base}")
    } else {
        base.to_string()
    }
}

pub fn auth_cookie_name_with_user(base: &str, authuser: &str, secure: bool) -> String {
    let name = if authuser == "0" || authuser.is_empty() {
        base.to_string()
    } else {
        format!("{base}_{authuser}")
    };
    if secure {
        format!("__Host-{name}")
    } else {
        name
    }
}

pub fn generate_csrf_token() -> String {
    use rand::Rng;
    let mut bytes = [0u8; 32];
    rand::rng().fill(&mut bytes);
    hex::encode(bytes)
}
