use sentry::protocol::{Url, Value};

pub(crate) fn scrub_value_map(values: &mut sentry::protocol::Map<String, Value>) {
    for (key, value) in values {
        scrub_value_for_key(key, value);
    }
}

pub(crate) fn scrub_tag_map(values: &mut sentry::protocol::Map<String, String>) {
    for (key, value) in values {
        if is_sensitive_key(key) {
            *value = "[Filtered]".to_string();
        } else {
            *value = scrub_sensitive_string(value);
        }
    }
}

pub(crate) fn scrub_value_for_key(key: &str, value: &mut Value) {
    if is_sensitive_key(key) {
        *value = Value::String("[Filtered]".to_string());
        return;
    }

    if let Value::String(raw) = value {
        if is_url_key(key) {
            *raw = sanitize_url_string(raw);
        } else {
            *raw = scrub_sensitive_string(raw);
        }
        return;
    }

    scrub_json_value(value);
}

pub(crate) fn scrub_json_value(value: &mut Value) {
    match value {
        Value::String(raw) => *raw = scrub_sensitive_string(raw),
        Value::Array(values) => {
            for child in values {
                scrub_json_value(child);
            }
        }
        Value::Object(values) => {
            for (key, child) in values {
                scrub_value_for_key(key, child);
            }
        }
        _ => {}
    }
}

pub(crate) fn scrub_url(mut url: Url) -> Url {
    let _ = url.set_username("");
    let _ = url.set_password(None);
    url.set_query(None);
    url.set_fragment(None);
    let path = sanitize_path_like(url.path());
    url.set_path(&path);
    url
}

pub(crate) fn sanitize_url_string(value: &str) -> String {
    match Url::parse(value) {
        Ok(url) => scrub_url(url).to_string(),
        Err(_) => scrub_sensitive_string(value),
    }
}

pub(crate) fn sanitize_path_like(value: &str) -> String {
    value
        .split('/')
        .map(|segment| {
            if should_redact_path_segment(segment) {
                "[id]"
            } else {
                segment
            }
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn should_redact_path_segment(segment: &str) -> bool {
    if segment.is_empty() {
        return false;
    }

    let lower = segment.to_ascii_lowercase();
    segment.contains('@')
        || is_uuid_like(&lower)
        || is_long_token_like(segment)
        || (looks_like_file_name(&lower) && !is_static_asset_name(&lower))
}

pub(crate) fn scrub_sensitive_string(value: &str) -> String {
    if looks_like_email(value) || looks_like_jwt(value) || contains_secret_marker(value) {
        "[Filtered]".to_string()
    } else {
        value.to_string()
    }
}

fn looks_like_email(value: &str) -> bool {
    let Some(at_index) = value.find('@') else {
        return false;
    };
    value[at_index..].contains('.')
}

fn looks_like_jwt(value: &str) -> bool {
    value.contains("eyJ") && value.matches('.').count() >= 2
}

fn contains_secret_marker(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    [
        "bearer ",
        "basic ",
        "token=",
        "secret=",
        "password=",
        "passwd=",
        "pwd=",
        "authorization",
        "cookie=",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

pub(crate) fn is_sensitive_key(key: &str) -> bool {
    let normalized = normalize_key(key);
    [
        "authorization",
        "cookie",
        "token",
        "secret",
        "password",
        "passwd",
        "pwd",
        "jwt",
        "api_key",
        "apikey",
        "private_key",
        "client_secret",
        "session",
        "refresh_token",
        "access_token",
        "id_token",
        "saml",
        "assertion",
        "credential",
        "mfa",
        "totp",
        "recovery",
        "email",
        "username",
        "file_name",
        "filename",
        "object_key",
        "objectkey",
    ]
    .iter()
    .any(|part| {
        normalized == *part
            || normalized.ends_with(&format!("_{part}"))
            || normalized.contains(&format!("{part}_"))
    })
}

pub(crate) fn is_url_key(key: &str) -> bool {
    let normalized = normalize_key(key);
    normalized == "url" || normalized == "href" || normalized.ends_with("_url")
}

fn normalize_key(key: &str) -> String {
    key.replace(['.', '-', ' '], "_").to_ascii_lowercase()
}

pub(crate) fn is_safe_header_key(key: &str) -> bool {
    matches!(
        normalize_key(key).as_str(),
        "accept" | "content_length" | "content_type" | "user_agent"
    )
}

fn is_uuid_like(value: &str) -> bool {
    value.len() == 36
        && value.chars().enumerate().all(|(index, char)| {
            matches!(index, 8 | 13 | 18 | 23) == (char == '-')
                && (char == '-' || char.is_ascii_hexdigit())
        })
}

fn is_long_token_like(value: &str) -> bool {
    value.len() >= 24
        && value
            .chars()
            .all(|char| char.is_ascii_alphanumeric() || matches!(char, '_' | '-'))
}

fn looks_like_file_name(value: &str) -> bool {
    value
        .rsplit_once('.')
        .is_some_and(|(_, ext)| !ext.is_empty() && ext.len() <= 12)
}

fn is_static_asset_name(value: &str) -> bool {
    [
        ".css", ".js", ".mjs", ".map", ".png", ".jpg", ".jpeg", ".gif", ".svg", ".ico", ".webp",
        ".avif", ".woff", ".woff2",
    ]
    .iter()
    .any(|ext| value.ends_with(ext))
}
