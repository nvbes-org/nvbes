use super::*;
use std::collections::HashMap;

fn key(byte: u8) -> String {
    STANDARD.encode([byte; 32])
}

fn load(values: &[(&str, String)]) -> Result<MfaKeys, MfaKeyError> {
    let map: HashMap<String, String> = values
        .iter()
        .map(|(name, value)| ((*name).to_owned(), value.clone()))
        .collect();
    MfaKeys::from_lookup(|name| map.get(name).cloned())
}

#[test]
fn active_key_is_required() {
    assert_eq!(load(&[]).err(), Some(MfaKeyError::Missing(ENCRYPTION_KEY)));
}

#[test]
fn active_key_defaults_to_version_one() {
    let keys = load(&[(ENCRYPTION_KEY, key(7))]).expect("keys");
    assert_eq!(keys.active_version, 1);
    assert_eq!(keys.active_key, [7; 32]);
    assert_eq!(keys.previous, None);
    assert_eq!(keys.crypto().expect("crypto").active_version(), 1);
}

#[test]
fn keys_must_be_32_bytes_of_base64() {
    for value in ["not-base64!", "AAAA", &STANDARD.encode([1u8; 31])] {
        assert_eq!(
            load(&[(ENCRYPTION_KEY, value.to_owned())]).err(),
            Some(MfaKeyError::Invalid(ENCRYPTION_KEY))
        );
    }
}

#[test]
fn versions_must_be_positive_integers() {
    for version in ["0", "-1", "one"] {
        assert_eq!(
            load(&[(ENCRYPTION_KEY, key(1)), (KEY_VERSION, version.to_owned())]).err(),
            Some(MfaKeyError::Invalid(KEY_VERSION))
        );
    }
}

#[test]
fn previous_key_requires_both_halves_and_a_distinct_version() {
    assert_eq!(
        load(&[(ENCRYPTION_KEY, key(1)), (PREVIOUS_ENCRYPTION_KEY, key(2))]).err(),
        Some(MfaKeyError::Missing(PREVIOUS_KEY_VERSION))
    );
    assert_eq!(
        load(&[(ENCRYPTION_KEY, key(1)), (PREVIOUS_KEY_VERSION, "1".into())]).err(),
        Some(MfaKeyError::Missing(PREVIOUS_ENCRYPTION_KEY))
    );
    assert_eq!(
        load(&[
            (ENCRYPTION_KEY, key(1)),
            (PREVIOUS_ENCRYPTION_KEY, key(2)),
            (PREVIOUS_KEY_VERSION, "1".into()),
        ])
        .err(),
        Some(MfaKeyError::Invalid(PREVIOUS_KEY_VERSION))
    );
}

#[test]
fn rotation_keeps_the_previous_key_readable() {
    let keys = load(&[
        (ENCRYPTION_KEY, key(1)),
        (KEY_VERSION, "2".into()),
        (PREVIOUS_ENCRYPTION_KEY, key(2)),
        (PREVIOUS_KEY_VERSION, "1".into()),
    ])
    .expect("rotation keys");
    assert_eq!(keys.previous, Some((1, [2; 32])));
    assert_eq!(keys.crypto().expect("crypto").active_version(), 2);
}
