use uuid::Uuid;

use super::{ListObjectsResponse, TrashListResponse};

#[test]
fn object_page_serializes_cursor_and_has_more() {
    let payload = serde_json::to_value(ListObjectsResponse {
        parent_id: Some(Uuid::from_u128(1)),
        objects: Vec::new(),
        next_cursor: Some("opaque-cursor".to_string()),
        has_more: true,
    })
    .expect("serializes");

    assert_eq!(payload["next_cursor"], "opaque-cursor");
    assert_eq!(payload["has_more"], true);
}

#[test]
fn trash_page_serializes_cursor_and_has_more() {
    let payload = serde_json::to_value(TrashListResponse {
        objects: Vec::new(),
        next_cursor: Some("opaque-cursor".to_string()),
        has_more: true,
    })
    .expect("serializes");

    assert_eq!(payload["next_cursor"], "opaque-cursor");
    assert_eq!(payload["has_more"], true);
}
