use crate::http::error::AppError;
use axum::http::HeaderMap;
use nvbes_region::{
    DataRegion, country_code_to_data_region,
    geo::{GeoLookupRequest, GeoResolver, parse_ip},
};

pub fn authorization_bearer_token(headers: &HeaderMap) -> Result<Option<String>, AppError> {
    let Some(auth_header) = headers.get("Authorization") else {
        return Ok(None);
    };
    let auth_str = auth_header
        .to_str()
        .map_err(|_| AppError::unauthorized("invalid_token", "Invalid Authorization header"))?;
    let token = auth_str.strip_prefix("Bearer ").ok_or_else(|| {
        AppError::unauthorized("invalid_token", "Authorization must use the Bearer scheme.")
    })?;
    if token.is_empty() {
        return Err(AppError::unauthorized(
            "invalid_token",
            "Bearer token is empty.",
        ));
    }
    Ok(Some(token.to_string()))
}

pub fn browser_session_token_with_authuser(
    headers: &HeaderMap,
    authuser: &str,
) -> Result<String, AppError> {
    if let Some(cookie_header) = headers.get("Cookie") {
        cookie_header
            .to_str()
            .map_err(|_| AppError::unauthorized("invalid_cookie", "Invalid Cookie header"))?;

        let secure_name = format!("__Host-session_{}=", authuser);
        let normal_name = format!("session_{}=", authuser);

        if authuser == "0" || authuser.is_empty() {
            if let Some(token) = cookie_value(
                headers,
                &[&secure_name, &normal_name, "__Host-session=", "session="],
            ) {
                return Ok(token);
            }
        } else {
            if let Some(token) = cookie_value(headers, &[&secure_name, &normal_name]) {
                return Ok(token);
            }
        }
    }

    Err(AppError::unauthorized(
        "missing_session_cookie",
        "No browser session cookie was found.",
    ))
}

pub fn bearer_token(headers: &HeaderMap) -> Result<String, AppError> {
    authorization_bearer_token(headers)?.ok_or_else(|| {
        AppError::unauthorized(
            "missing_token",
            "No Bearer token was found in Authorization.",
        )
    })
}

pub fn cookie_value(headers: &HeaderMap, names: &[&str]) -> Option<String> {
    let cookie_header = headers.get("Cookie")?;
    let cookie_str = cookie_header.to_str().ok()?;

    for cookie in cookie_str.split(';') {
        let cookie = cookie.trim();
        for name in names {
            if let Some(token) = cookie.strip_prefix(name) {
                return Some(token.to_string());
            }
        }
    }

    None
}

pub fn session_cookie_values(headers: &HeaderMap) -> Vec<String> {
    session_cookie_tokens(headers)
        .into_iter()
        .map(|cookie| cookie.token)
        .collect()
}

pub struct SessionCookieToken {
    pub authuser: String,
    pub token: String,
}

pub fn session_cookie_tokens(headers: &HeaderMap) -> Vec<SessionCookieToken> {
    let Some(cookie_header) = headers.get("Cookie") else {
        return Vec::new();
    };
    let Ok(cookie_str) = cookie_header.to_str() else {
        return Vec::new();
    };

    cookie_str
        .split(';')
        .filter_map(|cookie| {
            let (name, value) = cookie.trim().split_once('=')?;
            let authuser = session_cookie_authuser(name)?;
            Some(SessionCookieToken {
                authuser: authuser.to_string(),
                token: value.to_string(),
            })
        })
        .collect()
}

fn is_session_cookie_name(name: &str) -> bool {
    matches!(
        name,
        "__Host-session" | "session" | "__Host-token" | "token"
    ) || name.starts_with("__Host-session_")
        || name.starts_with("session_")
}

fn session_cookie_authuser(name: &str) -> Option<&str> {
    if let Some(suffix) = name.strip_prefix("__Host-session_") {
        return Some(suffix);
    }
    if let Some(suffix) = name.strip_prefix("session_") {
        return Some(suffix);
    }
    if is_session_cookie_name(name) {
        return Some("0");
    }
    None
}

pub fn client_ip(headers: &HeaderMap) -> Option<String> {
    nvbes_core::http::client_ip::client_ip(headers)
}

pub fn user_agent(headers: &HeaderMap) -> Option<String> {
    headers
        .get("User-Agent")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string())
}

pub fn gpc_enabled(headers: &HeaderMap) -> bool {
    headers
        .get("Sec-GPC")
        .and_then(|h| h.to_str().ok())
        .map(|v| v.trim() == "1")
        .unwrap_or(false)
}

pub fn region_from_headers(headers: &HeaderMap) -> Option<String> {
    let trusted_country_header = country_header(
        headers,
        &["CF-IPCountry", "X-Vercel-IP-Country", "X-AppEngine-Country"],
    );
    let ip = client_ip(headers).as_deref().and_then(parse_ip);
    let resolution = GeoResolver::default().resolve(GeoLookupRequest {
        ip,
        trusted_country_header,
        ..GeoLookupRequest::default()
    });

    resolution.location.map(|location| location.country_code)
}

pub fn country_header<'a>(headers: &'a HeaderMap, names: &[&str]) -> Option<&'a str> {
    names
        .iter()
        .find_map(|name| headers.get(*name).and_then(|h| h.to_str().ok()))
}

pub fn data_region_from_headers(headers: &HeaderMap) -> Option<String> {
    let country = region_from_headers(headers)?;
    country_code_to_data_region(&country).map(|region| region.as_str().to_string())
}

pub fn supported_data_regions() -> Vec<String> {
    [
        DataRegion::Eu,
        DataRegion::Us,
        DataRegion::Ch,
        DataRegion::Apac,
    ]
    .into_iter()
    .map(|region| region.as_str().to_string())
    .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        authorization_bearer_token, browser_session_token_with_authuser, region_from_headers,
        session_cookie_tokens, session_cookie_values,
    };
    use axum::http::{HeaderMap, HeaderValue};

    #[test]
    fn region_from_headers_prefers_cf_country() {
        let mut headers = HeaderMap::new();
        headers.insert("CF-IPCountry", HeaderValue::from_static("fr"));
        headers.insert("X-Vercel-IP-Country", HeaderValue::from_static("us"));

        assert_eq!(region_from_headers(&headers).as_deref(), Some("FR"));
    }

    #[test]
    fn region_from_headers_falls_back_to_vercel() {
        let mut headers = HeaderMap::new();
        headers.insert("X-Vercel-IP-Country", HeaderValue::from_static("us"));

        assert_eq!(region_from_headers(&headers).as_deref(), Some("US"));
    }

    #[test]
    fn session_cookie_values_extracts_base_and_authuser_sessions() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "Cookie",
            HeaderValue::from_static(
                "theme=dark; __Host-session=base; session_1=one; __Host-session_2=two; csrf_token=csrf",
            ),
        );

        assert_eq!(session_cookie_values(&headers), ["base", "one", "two"]);
    }

    #[test]
    fn session_cookie_tokens_extract_authuser_and_token() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "Cookie",
            HeaderValue::from_static(
                "theme=dark; __Host-session=base; session_1=one; __Host-session_2=two; csrf_token=csrf",
            ),
        );

        let cookies = session_cookie_tokens(&headers);
        assert_eq!(cookies.len(), 3);
        assert_eq!(cookies[0].authuser, "0");
        assert_eq!(cookies[0].token, "base");
        assert_eq!(cookies[1].authuser, "1");
        assert_eq!(cookies[1].token, "one");
        assert_eq!(cookies[2].authuser, "2");
        assert_eq!(cookies[2].token, "two");
    }

    #[test]
    fn browser_session_cookie_is_not_treated_as_a_bearer_token() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "Cookie",
            HeaderValue::from_static("__Host-session_1=v1.session.secret-value"),
        );

        assert_eq!(authorization_bearer_token(&headers).unwrap(), None);
        assert_eq!(
            browser_session_token_with_authuser(&headers, "1").unwrap(),
            "v1.session.secret-value"
        );
    }
}
