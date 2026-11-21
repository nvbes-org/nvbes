use axum::http::{HeaderMap, HeaderValue};
use uuid::Uuid;

use super::correlation_id;

#[test]
fn correlation_id_reuses_valid_header() {
    let id = Uuid::new_v4();
    let mut headers = HeaderMap::new();
    headers.insert(
        "x-correlation-id",
        HeaderValue::from_str(&id.to_string()).unwrap(),
    );
    assert_eq!(correlation_id(&headers), id);
}

#[test]
fn correlation_id_generates_when_missing_or_invalid() {
    let empty = HeaderMap::new();
    let generated = correlation_id(&empty);
    assert_ne!(generated, Uuid::nil());

    let mut headers = HeaderMap::new();
    headers.insert("x-correlation-id", HeaderValue::from_static("not-a-uuid"));
    let replaced = correlation_id(&headers);
    assert_ne!(replaced, Uuid::nil());
}
