use uuid::Uuid;

use super::MfaCrypto;

#[test]
fn sealed_totp_secret_is_bound_to_its_factor() {
    let crypto = MfaCrypto::with_rotation(1, [7; 32], None).unwrap();
    let factor_id = Uuid::new_v4();
    let sealed = crypto.seal(factor_id, "BASE32SECRET").unwrap();

    assert_eq!(
        crypto
            .open(
                factor_id,
                sealed.key_version,
                &sealed.ciphertext,
                &sealed.nonce
            )
            .unwrap(),
        "BASE32SECRET"
    );
    assert!(
        crypto
            .open(
                Uuid::new_v4(),
                sealed.key_version,
                &sealed.ciphertext,
                &sealed.nonce
            )
            .is_err()
    );
}

#[test]
fn rotation_reads_previous_but_writes_only_active_version() {
    let factor_id = Uuid::new_v4();
    let old = MfaCrypto::with_rotation(1, [3; 32], None).unwrap();
    let old_secret = old.seal(factor_id, "ROTATIONSECRET").unwrap();
    let rotating = MfaCrypto::with_rotation(2, [4; 32], Some((1, [3; 32]))).unwrap();

    assert_eq!(rotating.active_version(), 2);
    assert_eq!(
        rotating
            .open(factor_id, 1, &old_secret.ciphertext, &old_secret.nonce)
            .unwrap(),
        "ROTATIONSECRET"
    );
    assert_eq!(
        rotating
            .seal(factor_id, "ROTATIONSECRET")
            .unwrap()
            .key_version,
        2
    );
    assert!(
        rotating
            .open(factor_id, 9, &old_secret.ciphertext, &old_secret.nonce)
            .is_err()
    );
}

#[test]
fn with_rotation_rejects_non_positive_or_colliding_versions() {
    assert!(MfaCrypto::with_rotation(0, [1; 32], None).is_err());
    assert!(MfaCrypto::with_rotation(-3, [1; 32], None).is_err());
    assert!(MfaCrypto::with_rotation(1, [1; 32], Some((0, [2; 32]))).is_err());
    assert!(MfaCrypto::with_rotation(2, [1; 32], Some((-1, [2; 32]))).is_err());
    assert!(MfaCrypto::with_rotation(2, [1; 32], Some((2, [2; 32]))).is_err());
}

#[test]
fn open_rejects_invalid_nonce_length() {
    let crypto = MfaCrypto::with_rotation(1, [9; 32], None).unwrap();
    let factor_id = Uuid::new_v4();
    let sealed = crypto.seal(factor_id, "SECRET").unwrap();
    assert!(
        crypto
            .open(factor_id, sealed.key_version, &sealed.ciphertext, &[])
            .is_err()
    );
    assert!(
        crypto
            .open(
                factor_id,
                sealed.key_version,
                &sealed.ciphertext,
                &[0_u8; 11]
            )
            .is_err()
    );
}
