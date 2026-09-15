use openssl::{pkey::PKey, rsa::Rsa};
use proptest::prelude::*;
use std::sync::LazyLock;
use uuid::Uuid;

use super::{TokenService, validate_scope};
use crate::tokens_config::TokenConfig;

static SERVICE: LazyLock<TokenService> = LazyLock::new(|| {
    let private = PKey::from_rsa(Rsa::generate(2048).unwrap()).unwrap();
    TokenService::new(
        TokenConfig::from_values(
            "test",
            "http://identity.test".into(),
            "identity-key-1".into(),
            String::from_utf8(private.private_key_to_pem_pkcs8().unwrap()).unwrap(),
            String::from_utf8(private.public_key_to_pem().unwrap()).unwrap(),
            "nvbes-account-service".into(),
        )
        .unwrap(),
    )
    .unwrap()
});

proptest! {
    #[test]
    fn validate_scope_never_panics_on_arbitrary_strings(s in ".*") {
        let _ = validate_scope(&s);
    }

    #[test]
    fn validate_scope_accepts_valid_scope_tokens(
        tokens in prop::collection::vec("[a-zA-Z0-9:_.\\-]{1,32}", 1..8)
    ) {
        let scope = tokens.join(" ");
        prop_assert!(validate_scope(&scope).is_ok());
    }

    #[test]
    fn validate_scope_rejects_empty_or_whitespace_padding(
        tokens in prop::collection::vec("[a-zA-Z0-9:_.\\-]{1,16}", 1..4),
        leading_space in "[ ]{1,3}",
        trailing_space in "[ ]{0,3}"
    ) {
        let padded = format!("{}{}{}", leading_space, tokens.join(" "), trailing_space);
        prop_assert!(validate_scope(&padded).is_err());
        prop_assert!(validate_scope("").is_err());
    }

    #[test]
    fn validate_scope_rejects_disallowed_characters(
        prefix in "[a-zA-Z0-9]{1,8}",
        bad_char in "[!@#$%^&*()+=<>{}\\[\\]|;,'\"`~]",
        suffix in "[a-zA-Z0-9]{1,8}"
    ) {
        let invalid = format!("{prefix}{bad_char}{suffix}");
        prop_assert!(validate_scope(&invalid).is_err());
    }

    #[test]
    fn token_verify_never_panics_on_arbitrary_input(
        token in ".*",
        aud in "[a-zA-Z0-9\\-_]{1,32}"
    ) {
        let _ = SERVICE.verify(&token, &aud);
    }

    #[test]
    fn token_issue_and_verify_roundtrip(
        principal_id in prop::num::u128::ANY.prop_map(Uuid::from_u128),
        session_id in prop::num::u128::ANY.prop_map(Uuid::from_u128),
        scope_token in "[a-z]{3,8}:[a-z]{3,8}"
    ) {
        let token = SERVICE.issue(
            principal_id,
            session_id,
            "nvbes-account-service",
            &scope_token,
            vec!["pwd".into()]
        ).unwrap();

        let claims = SERVICE.verify(&token, "nvbes-account-service").unwrap();
        prop_assert_eq!(claims.sub, principal_id.to_string());
        prop_assert_eq!(claims.sid, session_id.to_string());
        prop_assert_eq!(claims.scope, scope_token);
        prop_assert_eq!(claims.aud, "nvbes-account-service");

        // Reject wrong audience fail-closed
        prop_assert!(SERVICE.verify(&token, "wrong-audience").is_err());
    }

    #[test]
    fn token_tampered_signature_fails(
        idx in 0usize..30,
        sub in prop::num::u128::ANY.prop_map(Uuid::from_u128),
        sid in prop::num::u128::ANY.prop_map(Uuid::from_u128)
    ) {
        let token = SERVICE.issue(
            sub,
            sid,
            "nvbes-account-service",
            "account:read",
            vec!["pwd".into()]
        ).unwrap();

        let mut bytes = token.into_bytes();
        let len = bytes.len();
        let target = len.saturating_sub(1 + (idx % 20));
        bytes[target] ^= 0x42; // flip bits in signature or payload
        let tampered = String::from_utf8_lossy(&bytes);

        prop_assert!(SERVICE.verify(&tampered, "nvbes-account-service").is_err());
    }
}
