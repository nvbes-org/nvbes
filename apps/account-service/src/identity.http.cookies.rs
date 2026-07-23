use crate::http::error::AppError;
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

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
        .map_err(|e| AppError::internal("cookie_header_invalid", format!("{}", e)))
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
        .map_err(|e| AppError::internal("cookie_header_invalid", format!("{}", e)))
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

pub fn generate_csrf_token(session_token: &str, secret: &str) -> String {
    use rand::Rng;
    let mut bytes = [0u8; 32];
    rand::rng().fill(&mut bytes);
    let nonce = hex::encode(bytes);
    let signature = csrf_signature(session_token, &nonce, secret);
    format!("v1.{nonce}.{signature}")
}

pub fn verify_csrf_token(csrf_token: &str, session_token: &str, secret: &str) -> bool {
    let mut parts = csrf_token.split('.');
    let Some("v1") = parts.next() else {
        return false;
    };
    let Some(nonce) = parts.next() else {
        return false;
    };
    let Some(signature) = parts.next() else {
        return false;
    };
    if parts.next().is_some() || nonce.len() != 64 || hex::decode(nonce).is_err() {
        return false;
    }

    let Ok(signature_bytes) = hex::decode(signature) else {
        return false;
    };

    csrf_mac(session_token, nonce, secret)
        .verify_slice(&signature_bytes)
        .is_ok()
}

fn csrf_signature(session_token: &str, nonce: &str, secret: &str) -> String {
    hex::encode(
        csrf_mac(session_token, nonce, secret)
            .finalize()
            .into_bytes(),
    )
}

fn csrf_mac(session_token: &str, nonce: &str, secret: &str) -> HmacSha256 {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .expect("HMAC accepts signing keys of any length");
    mac.update(b"nvbes.csrf.v1");
    mac.update(session_token.len().to_string().as_bytes());
    mac.update(b"!");
    mac.update(session_token.as_bytes());
    mac.update(b"!");
    mac.update(nonce.len().to_string().as_bytes());
    mac.update(b"!");
    mac.update(nonce.as_bytes());
    mac
}

#[cfg(test)]
mod tests {
    use super::{generate_csrf_token, verify_csrf_token};

    #[test]
    fn csrf_token_is_bound_to_session_token() {
        let token = generate_csrf_token("session-a", "secret");

        assert!(verify_csrf_token(&token, "session-a", "secret"));
        assert!(!verify_csrf_token(&token, "session-b", "secret"));
    }

    #[test]
    fn csrf_token_rejects_unsigned_legacy_values() {
        assert!(!verify_csrf_token("plain-random", "session-a", "secret"));
    }
}
