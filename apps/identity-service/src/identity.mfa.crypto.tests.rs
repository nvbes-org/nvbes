use uuid::Uuid;

use super::MfaCrypto;

#[test]
fn recovery_encryption_binds_domain_principal_challenge_and_rotates() {
    let principal = Uuid::new_v4();
    let challenge = Uuid::new_v4();
    let old = MfaCrypto::with_rotation(1, [3; 32], None).unwrap();
    let sealed = old
        .seal_recovery(principal, challenge, "credential-email")
        .unwrap();
    let rotating = MfaCrypto::with_rotation(2, [4; 32], Some((1, [3; 32]))).unwrap();
    assert_eq!(
        rotating
            .open_recovery(principal, challenge, 1, &sealed.ciphertext, &sealed.nonce)
            .unwrap(),
        "credential-email"
    );
    assert!(
        old.open(challenge, 1, &sealed.ciphertext, &sealed.nonce)
            .is_err()
    );
    for (owner, id) in [(Uuid::new_v4(), challenge), (principal, Uuid::new_v4())] {
        assert!(
            old.open_recovery(owner, id, 1, &sealed.ciphertext, &sealed.nonce)
                .is_err()
        );
    }
    let retired = MfaCrypto::with_rotation(2, [4; 32], None).unwrap();
    assert!(
        retired
            .open_recovery(principal, challenge, 1, &sealed.ciphertext, &sealed.nonce)
            .is_err()
    );
    let mut corrupt = sealed.ciphertext;
    corrupt[0] ^= 1;
    assert!(
        old.open_recovery(principal, challenge, 1, &corrupt, &sealed.nonce)
            .is_err()
    );
    assert_eq!(
        rotating
            .seal_recovery(principal, challenge, "next")
            .unwrap()
            .key_version,
        2
    );
}

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
}
