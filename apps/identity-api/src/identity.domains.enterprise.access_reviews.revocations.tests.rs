use super::{ensure_ownerless_count_is_zero, parse_role_subject, parse_uuid_subject};
use uuid::Uuid;

#[test]
fn parse_role_subject_accepts_snapshot_subject_with_workspace_suffix() {
    let principal_id = Uuid::new_v4();
    let workspace_id = Uuid::new_v4();
    let subject = format!("{principal_id}:{workspace_id}");

    assert_eq!(parse_role_subject(&subject).unwrap(), principal_id);
}

#[test]
fn parse_role_subject_accepts_plain_principal_id() {
    let principal_id = Uuid::new_v4();

    assert_eq!(
        parse_role_subject(&principal_id.to_string()).unwrap(),
        principal_id
    );
}

#[test]
fn parse_uuid_subject_rejects_invalid_subjects() {
    let err = parse_uuid_subject("not-a-uuid").expect_err("invalid subjects must fail");

    assert_eq!(err.code, "invalid_access_review_item");
}

#[test]
fn last_owner_guard_allows_when_no_workspace_would_be_ownerless() {
    ensure_ownerless_count_is_zero(0).expect("no ownerless workspaces should pass");
}

#[test]
fn last_owner_guard_blocks_ownerless_workspace() {
    let err = ensure_ownerless_count_is_zero(1).expect_err("ownerless workspace should fail");

    assert_eq!(err.code, "last_owner_removal");
}
