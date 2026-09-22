use axum::http::{HeaderMap, HeaderValue, header::SET_COOKIE};

pub const SESSION_COOKIE_NAME: &str = "nvbes_sid";
pub const SESSION_TTL_HOURS: i64 = 1;

pub fn session_token_from_headers(headers: &HeaderMap) -> Option<String> {
    if let Some(token) = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|token| !token.is_empty())
    {
        return Some(token.to_string());
    }

    headers
        .get(axum::http::header::COOKIE)
        .and_then(|value| value.to_str().ok())
        .and_then(session_token_from_cookie_header)
}

fn session_token_from_cookie_header(header: &str) -> Option<String> {
    header.split(';').find_map(|part| {
        let part = part.trim();
        part.strip_prefix(SESSION_COOKIE_NAME)
            .and_then(|rest| rest.strip_prefix('='))
            .map(str::trim)
            .filter(|token| !token.is_empty())
            .map(str::to_string)
    })
}

pub fn set_session_cookie(token: &str, secure: bool) -> HeaderValue {
    let mut cookie = format!(
        "{SESSION_COOKIE_NAME}={token}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}",
        SESSION_TTL_HOURS * 3600
    );
    if secure {
        cookie.push_str("; Secure");
    }
    HeaderValue::from_str(&cookie).expect("session cookie is ASCII")
}

pub fn clear_session_cookie(secure: bool) -> HeaderValue {
    let mut cookie = format!("{SESSION_COOKIE_NAME}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0");
    if secure {
        cookie.push_str("; Secure");
    }
    HeaderValue::from_str(&cookie).expect("clear cookie is ASCII")
}

pub fn append_session_cookie(headers: &mut HeaderMap, token: &str, secure: bool) {
    headers.insert(SET_COOKIE, set_session_cookie(token, secure));
}

pub fn append_cleared_session_cookie(headers: &mut HeaderMap, secure: bool) {
    headers.insert(SET_COOKIE, clear_session_cookie(secure));
}

/// Accept a relative `/oauth/authorize?...` path or an absolute URL under the issuer.
pub fn validate_return_to(return_to: &str, issuer: &str) -> Option<String> {
    let return_to = return_to.trim();
    if return_to.is_empty() || return_to.len() > 2048 {
        return None;
    }
    if return_to.starts_with("/oauth/authorize?") || return_to == "/oauth/authorize" {
        return Some(return_to.to_string());
    }
    let issuer = issuer.trim_end_matches('/');
    if return_to.starts_with(issuer) {
        let path = &return_to[issuer.len()..];
        if path.starts_with("/oauth/authorize?") || path == "/oauth/authorize" {
            return Some(return_to.to_string());
        }
    }
    None
}

pub fn authorize_return_to(uri: &axum::http::Uri) -> String {
    match uri.query() {
        Some(query) => format!("/oauth/authorize?{query}"),
        None => "/oauth/authorize".to_string(),
    }
}

#[cfg(test)]
#[path = "identity.session.tests.rs"]
mod tests;
