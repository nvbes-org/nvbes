use proptest::prelude::*;
use uuid::Uuid;

use super::MfaCrypto;

proptest! {
    #[test]
    fn mfa_open_never_panics_on_arbitrary_input(
        factor_id in prop::num::u128::ANY.prop_map(Uuid::from_u128),
        key_version in any::<i16>(),
        ciphertext in prop::collection::vec(any::<u8>(), 0..256),
        nonce in prop::collection::vec(any::<u8>(), 0..32)
    ) {
        let crypto = MfaCrypto::with_rotation(1, [42; 32], None).unwrap();
        let _ = crypto.open(factor_id, key_version, &ciphertext, &nonce);
    }

    #[test]
    fn mfa_seal_and_open_roundtrips(
        factor_id in prop::num::u128::ANY.prop_map(Uuid::from_u128),
        secret in "\\PC*"
    ) {
        let crypto = MfaCrypto::with_rotation(1, [15; 32], None).unwrap();
        let sealed = crypto.seal(factor_id, &secret).unwrap();
        let decrypted = crypto.open(
            factor_id,
            sealed.key_version,
            &sealed.ciphertext,
            &sealed.nonce
        ).unwrap();

        prop_assert_eq!(decrypted, secret);
    }

    #[test]
    fn mfa_open_rejects_tampered_ciphertext_or_nonce(
        factor_id in prop::num::u128::ANY.prop_map(Uuid::from_u128),
        secret in "[A-Z0-9]{16,64}",
        corrupt_idx in 0usize..256
    ) {
        let crypto = MfaCrypto::with_rotation(1, [99; 32], None).unwrap();
        let sealed = crypto.seal(factor_id, &secret).unwrap();

        // Tamper with ciphertext
        let mut tampered_ct = sealed.ciphertext.clone();
        if !tampered_ct.is_empty() {
            let idx = corrupt_idx % tampered_ct.len();
            tampered_ct[idx] ^= 0x01;
            prop_assert!(crypto.open(factor_id, sealed.key_version, &tampered_ct, &sealed.nonce).is_err());
        }

        // Tamper with nonce
        let mut tampered_nonce = sealed.nonce;
        let nonce_idx = corrupt_idx % tampered_nonce.len();
        tampered_nonce[nonce_idx] ^= 0x01;
        prop_assert!(crypto.open(factor_id, sealed.key_version, &sealed.ciphertext, &tampered_nonce).is_err());
    }

    #[test]
    fn mfa_open_rejects_wrong_factor_or_version(
        factor_id_1 in prop::num::u128::ANY.prop_map(Uuid::from_u128),
        factor_id_2 in prop::num::u128::ANY.prop_map(Uuid::from_u128),
        secret in "[A-Z0-9]{16,32}"
    ) {
        prop_assume!(factor_id_1 != factor_id_2);
        let crypto = MfaCrypto::with_rotation(1, [77; 32], None).unwrap();
        let sealed = crypto.seal(factor_id_1, &secret).unwrap();

        // Wrong factor ID (AAD mismatch)
        prop_assert!(crypto.open(factor_id_2, sealed.key_version, &sealed.ciphertext, &sealed.nonce).is_err());

        // Unknown key version
        prop_assert!(crypto.open(factor_id_1, 999, &sealed.ciphertext, &sealed.nonce).is_err());
    }
}
