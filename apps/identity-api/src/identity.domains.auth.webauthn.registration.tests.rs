use super::{normalize_registration_kind, shape_registration_options};
use serde_json::json;

#[test]
fn normalize_registration_kind_defaults_to_passkey() {
    assert_eq!(normalize_registration_kind(None).unwrap(), "passkey");
}

#[test]
fn normalize_registration_kind_accepts_security_key() {
    assert_eq!(
        normalize_registration_kind(Some("security_key".to_string())).unwrap(),
        "security_key"
    );
}

#[test]
fn normalize_registration_kind_rejects_unknown_values() {
    let error = normalize_registration_kind(Some("roaming".to_string())).unwrap_err();

    assert_eq!(error.code, "webauthn_registration_kind_invalid");
}

#[test]
fn shape_registration_options_sets_cross_platform_attachment_for_security_keys() {
    let options = shape_registration_options(json!({ "publicKey": {} }), "security_key");

    assert_eq!(
        options["publicKey"]["authenticatorSelection"]["authenticatorAttachment"],
        "cross-platform"
    );
}
