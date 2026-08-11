use super::parse_message_id;

#[test]
fn queue_payload_is_exactly_an_optional_whitespace_wrapped_uuid() {
    let id = uuid::Uuid::new_v4();
    assert_eq!(parse_message_id(format!("  {id}\n").as_bytes()), Some(id));
    assert!(parse_message_id(b"not-a-message-id").is_none());
    assert!(parse_message_id(&[0xff]).is_none());
}
