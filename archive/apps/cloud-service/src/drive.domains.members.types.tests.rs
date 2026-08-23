use super::{InvitationListResponse, MemberListResponse};

#[test]
fn member_list_serializes_member_pagination_metadata() {
    let payload = serde_json::to_value(MemberListResponse {
        members: Vec::new(),
        members_next_cursor: Some("opaque-cursor".to_string()),
        members_has_more: true,
    })
    .expect("serializes");

    assert_eq!(payload["members_next_cursor"], "opaque-cursor");
    assert_eq!(payload["members_has_more"], true);
}

#[test]
fn invitation_list_serializes_pagination_metadata() {
    let payload = serde_json::to_value(InvitationListResponse {
        invitations: Vec::new(),
        next_cursor: Some("opaque-cursor".to_string()),
        has_more: true,
    })
    .expect("serializes");

    assert_eq!(payload["next_cursor"], "opaque-cursor");
    assert_eq!(payload["has_more"], true);
}
