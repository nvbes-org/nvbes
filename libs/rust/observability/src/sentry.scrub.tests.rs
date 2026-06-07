use sentry::protocol::{Breadcrumb, Event, Request, User, Value};

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
