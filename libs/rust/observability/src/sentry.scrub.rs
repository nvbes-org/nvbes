use sentry::protocol::{Breadcrumb, Event, Log, Request, Stacktrace, Url, Value};

pub(crate) fn scrub_event(mut event: Event<'static>) -> Option<Event<'static>> {
    event.user = None;

    if let Some(request) = event.request.as_mut() {
        scrub_request(request);
    }

    if let Some(message) = event.message.as_mut() {
        *message = scrub_sensitive_string(message);
    }

    if let Some(logentry) = event.logentry.as_mut() {
        logentry.message = scrub_sensitive_string(&logentry.message);
        for param in &mut logentry.params {
            scrub_json_value(param);
        }
    }

    if let Some(transaction) = event.transaction.as_mut() {
        *transaction = sanitize_path_like(transaction);
    }

    event.breadcrumbs.values = event
        .breadcrumbs
        .values
        .into_iter()
        .filter_map(scrub_breadcrumb)
        .collect();

    for exception in &mut event.exception.values {
        if let Some(value) = exception.value.as_mut() {
            *value = scrub_sensitive_string(value);
        }
        if let Some(stacktrace) = exception.stacktrace.as_mut() {
            scrub_stacktrace(stacktrace);
        }
        if let Some(stacktrace) = exception.raw_stacktrace.as_mut() {
            scrub_stacktrace(stacktrace);
        }
    }

    if let Some(stacktrace) = event.stacktrace.as_mut() {
        scrub_stacktrace(stacktrace);
    }

    scrub_value_map(&mut event.extra);
    scrub_tag_map(&mut event.tags);

    Some(event)
}

pub(crate) fn scrub_breadcrumb(mut breadcrumb: Breadcrumb) -> Option<Breadcrumb> {
    if breadcrumb
        .category
        .as_deref()
        .is_some_and(|category| category.starts_with("console"))
    {
        return None;
    }

    if let Some(message) = breadcrumb.message.as_mut() {
        *message = scrub_sensitive_string(message);
    }
    scrub_value_map(&mut breadcrumb.data);

    Some(breadcrumb)
}

pub(crate) fn scrub_log(mut log: Log) -> Option<Log> {
    log.body = scrub_sensitive_string(&log.body);

    for (key, attribute) in &mut log.attributes {
        scrub_value_for_key(key, &mut attribute.0);
    }

    Some(log)
}

fn scrub_request(request: &mut Request) {
    request.url = request.url.take().map(scrub_url);
    request.query_string = None;
    request.cookies = None;
    request.data = None;
    request.env.clear();
    request.headers.retain(|key, value| {
        if !is_safe_header_key(key) || is_sensitive_key(key) {
            return false;
        }
        *value = scrub_sensitive_string(value);
        true
    });
}

fn scrub_stacktrace(stacktrace: &mut Stacktrace) {
    stacktrace.registers.clear();
    for frame in &mut stacktrace.frames {
        frame.abs_path = None;
        frame.context_line = None;
        frame.pre_context.clear();
        frame.post_context.clear();
        frame.vars.clear();
    }
}

fn scrub_value_map(values: &mut sentry::protocol::Map<String, Value>) {
    for (key, value) in values {
        scrub_value_for_key(key, value);
    }
}

fn scrub_tag_map(values: &mut sentry::protocol::Map<String, String>) {
    for (key, value) in values {
        if is_sensitive_key(key) {
            *value = "[Filtered]".to_string();
        } else {
            *value = scrub_sensitive_string(value);
        }
    }
}

fn scrub_value_for_key(key: &str, value: &mut Value) {
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

fn scrub_json_value(value: &mut Value) {
    match value {
        Value::String(raw) => {
            *raw = scrub_sensitive_string(raw);
        }
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

fn scrub_url(mut url: Url) -> Url {
    let _ = url.set_username("");
    let _ = url.set_password(None);
    url.set_query(None);
    url.set_fragment(None);

    let path = sanitize_path_like(url.path());
    url.set_path(&path);

    url
}

fn sanitize_url_string(value: &str) -> String {
    match Url::parse(value) {
        Ok(url) => scrub_url(url).to_string(),
        Err(_) => scrub_sensitive_string(value),
    }
}

fn sanitize_path_like(value: &str) -> String {
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

fn scrub_sensitive_string(value: &str) -> String {
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

fn is_sensitive_key(key: &str) -> bool {
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

fn is_url_key(key: &str) -> bool {
    let normalized = normalize_key(key);
    normalized == "url" || normalized == "href" || normalized.ends_with("_url")
}

fn normalize_key(key: &str) -> String {
    key.replace(['.', '-', ' '], "_").to_ascii_lowercase()
}

fn is_safe_header_key(key: &str) -> bool {
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

#[cfg(test)]
mod tests {
    use sentry::protocol::{Request, User};

    use super::*;

    #[test]
    fn scrub_event_removes_user_and_request_payloads() {
        let mut request = Request {
            url: Some(
                "https://api.nvbes.fr/users/ada@example.com?token=secret"
                    .parse()
                    .expect("valid test URL"),
            ),
            query_string: Some("token=secret".to_string()),
            cookies: Some("sid=secret".to_string()),
            data: Some(r#"{"email":"ada@example.com"}"#.to_string()),
            ..Default::default()
        };
        request
            .headers
            .insert("Authorization".to_string(), "Bearer secret".to_string());
        request
            .headers
            .insert("content-type".to_string(), "application/json".to_string());

        let event = Event {
            user: Some(User {
                email: Some("ada@example.com".to_string()),
                ..Default::default()
            }),
            message: Some("failed for ada@example.com".to_string()),
            request: Some(request),
            ..Default::default()
        };

        let scrubbed = scrub_event(event).expect("scrubbed event");
        let request = scrubbed.request.expect("scrubbed request");

        assert!(scrubbed.user.is_none());
        assert_eq!(scrubbed.message.as_deref(), Some("[Filtered]"));
        assert_eq!(
            request.url.as_ref().map(ToString::to_string).as_deref(),
            Some("https://api.nvbes.fr/users/[id]")
        );
        assert!(request.query_string.is_none());
        assert!(request.cookies.is_none());
        assert!(request.data.is_none());
        assert!(!request.headers.contains_key("Authorization"));
        assert_eq!(
            request.headers.get("content-type").map(String::as_str),
            Some("application/json")
        );
    }

    #[test]
    fn scrub_breadcrumb_drops_console_and_redacts_sensitive_data() {
        let console = Breadcrumb {
            category: Some("console".to_string()),
            message: Some("ada@example.com".to_string()),
            ..Default::default()
        };
        assert!(scrub_breadcrumb(console).is_none());

        let mut breadcrumb = Breadcrumb {
            category: Some("fetch".to_string()),
            ..Default::default()
        };
        breadcrumb.data.insert(
            "url".to_string(),
            Value::String("https://api.nvbes.fr/files/report.pdf?token=secret".to_string()),
        );
        breadcrumb.data.insert(
            "authorization".to_string(),
            Value::String("Bearer secret".to_string()),
        );

        let scrubbed = scrub_breadcrumb(breadcrumb).expect("scrubbed breadcrumb");
        assert_eq!(
            scrubbed.data.get("url").and_then(Value::as_str),
            Some("https://api.nvbes.fr/files/[id]")
        );
        assert_eq!(
            scrubbed.data.get("authorization").and_then(Value::as_str),
            Some("[Filtered]")
        );
    }
}
