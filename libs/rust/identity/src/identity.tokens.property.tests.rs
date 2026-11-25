use proptest::prelude::*;
use std::sync::LazyLock;

use super::{TokenService, tests};
use crate::tokens_policy::{ACCOUNT_AUDIENCE, validate_scopes};

static SERVICE: LazyLock<TokenService> =
    LazyLock::new(|| TokenService::new(tests::config()).unwrap());

proptest! {
    #[test]
    fn validate_scopes_never_panics_on_arbitrary_strings(s in ".*") {
        let _ = validate_scopes(ACCOUNT_AUDIENCE, &s);
    }

    #[test]
    fn validate_scopes_accepts_known_account_scopes(
        indexes in prop::collection::vec(0usize..4, 1..4)
    ) {
        let allowed = ["account:read", "account:write", "account:export", "account:close"];
        let mut scopes = indexes
            .into_iter()
            .map(|index| allowed[index])
            .collect::<Vec<_>>();
        scopes.sort_unstable();
        scopes.dedup();
        prop_assume!(!scopes.is_empty());
        prop_assert!(validate_scopes(ACCOUNT_AUDIENCE, &scopes.join(" ")).is_ok());
    }

    #[test]
    fn validate_scopes_rejects_empty_or_whitespace_padding(
        leading_space in "[ ]{1,3}",
        trailing_space in "[ ]{0,3}"
    ) {
        let padded = format!("{leading_space}account:read{trailing_space}");
        prop_assert!(validate_scopes(ACCOUNT_AUDIENCE, &padded).is_err());
        prop_assert!(validate_scopes(ACCOUNT_AUDIENCE, "").is_err());
    }

    #[test]
    fn validate_scopes_rejects_disallowed_characters(
        bad_char in "[!@#$%^&*()+=<>{}\\[\\]|;,'\"`~]"
    ) {
        let invalid = format!("account{bad_char}read");
        prop_assert!(validate_scopes(ACCOUNT_AUDIENCE, &invalid).is_err());
    }

    #[test]
    fn token_verify_never_panics_on_arbitrary_input(
        token in ".*",
        aud in "[a-zA-Z0-9\\-_]{1,32}"
    ) {
        let _ = SERVICE.verify(&token, &aud);
    }

    #[test]
    fn token_sign_grant_and_verify_roundtrip(_seed in 0u8..16) {
        let grant = tests::grant();
        let response = SERVICE.sign_grant(&grant).unwrap();
        let claims = SERVICE
            .verify(&response.access_token, "nvbes-account-service")
            .unwrap();
        prop_assert_eq!(claims.sub, grant.principal_id.to_string());
        prop_assert_eq!(claims.sid, grant.session_id.to_string());
        prop_assert_eq!(claims.scope, "account:read");
        prop_assert_eq!(claims.aud, "nvbes-account-service");
        prop_assert!(SERVICE.verify(&response.access_token, "wrong-audience").is_err());
    }

    #[test]
    fn token_tampered_signature_fails(idx in 0usize..30) {
        let response = SERVICE.sign_grant(&tests::grant()).unwrap();
        let mut bytes = response.access_token.into_bytes();
        let len = bytes.len();
        let target = len.saturating_sub(1 + (idx % 20));
        bytes[target] ^= 0x42;
        let tampered = String::from_utf8_lossy(&bytes);
        prop_assert!(SERVICE.verify(&tampered, "nvbes-account-service").is_err());
    }
}
