use super::*;
use nvbes_identity_sdk::pinned_keys::PinnedKeySet;

#[test]
fn pinned_keys_expire_exactly_and_never_fall_back_for_an_unknown_kid() {
    let overlap = json!([{"kid":"old", "public_key_pem":keys().1, "accept_until":100}]);
    let ring = PinnedKeySet::new("active", &keys().1, &overlap.to_string()).unwrap();
    assert!(ring.key(Some("old"), 99).is_ok());
    assert!(ring.key(Some("old"), 100).is_err());
    assert!(ring.key(Some("active"), 100).is_ok());
    assert!(ring.key(Some("unknown"), 99).is_err());
    assert!(ring.key(None, 99).is_err());
}

#[test]
fn invalid_overlap_configuration_never_silently_disables_rotation() {
    let entry = json!({"kid":"old", "public_key_pem":keys().1, "accept_until":100});
    for overlap in [
        "".to_string(), "null".into(), "{".into(),
        json!([entry.clone(),entry.clone()]).to_string(),
        json!([entry.clone(),entry.clone(),entry.clone(),entry.clone()]).to_string(),
        json!([{"kid":"active","public_key_pem":keys().1,"accept_until":100}]).to_string(),
        json!([{"kid":"old","public_key_pem":keys().1,"accept_until":0}]).to_string(),
        json!([{"kid":"old","public_key_pem":"invalid","accept_until":100}]).to_string(),
        json!([{"kid":"old","public_key_pem":keys().1,"accept_until":100,"private_key_pem":"forbidden"}]).to_string(),
        " ".repeat(65_537),
    ] {
        assert!(PinnedKeySet::new("active", &keys().1, &overlap).is_err());
    }
}

#[test]
fn account_accepts_an_overlapping_signer_but_rejects_it_after_retirement() {
    let token = sign(&claims());
    let mut config = config();
    config.token_key_id = "next-key".into();
    config.token_verification_keys = json!([{"kid":"identity-test", "public_key_pem":keys().1, "accept_until":chrono::Utc::now().timestamp()+60}]).to_string();
    assert!(
        TokenVerifier::new(&config)
            .unwrap()
            .validated(&token)
            .is_ok()
    );
    config.token_verification_keys =
        json!([{"kid":"identity-test", "public_key_pem":keys().1, "accept_until":1}]).to_string();
    assert!(
        TokenVerifier::new(&config)
            .unwrap()
            .validated(&token)
            .is_err()
    );
}
