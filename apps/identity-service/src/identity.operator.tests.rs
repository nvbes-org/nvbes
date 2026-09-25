use crate::tokens::{OPERATOR_AUDIENCE, OPERATOR_ROLE};

#[test]
fn operator_contract_constants_match_platform_operations() {
    assert_eq!(OPERATOR_AUDIENCE, "platform-operations");
    assert_eq!(OPERATOR_ROLE, "platform_owner");
}
