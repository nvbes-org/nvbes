use proptest::prelude::*;

use super::validation::{
    validate_email, validate_https_url, validate_identifier, validate_money, validate_text,
};

proptest! {
    #[test]
    fn validate_identifier_never_panics_on_arbitrary_input(s in ".*") {
        let _ = validate_identifier("test_field", &s, 64);
    }

    #[test]
    fn validate_identifier_accepts_valid_tokens(
        token in "[a-zA-Z0-9_\\-:]{1,32}"
    ) {
        prop_assert!(validate_identifier("test_field", &token, 32).is_ok());
    }

    #[test]
    fn validate_identifier_rejects_empty_or_excess_length(
        token in "[a-zA-Z0-9]{33,64}"
    ) {
        prop_assert!(validate_identifier("test_field", "", 32).is_err());
        prop_assert!(validate_identifier("test_field", &token, 32).is_err());
    }

    #[test]
    fn validate_identifier_rejects_disallowed_characters(
        prefix in "[a-zA-Z0-9]{1,8}",
        bad_char in "[ !@#$%^&*()+=<>{}\\[\\]|;,'\"`~/?\\\\]",
        suffix in "[a-zA-Z0-9]{1,8}"
    ) {
        let invalid = format!("{prefix}{bad_char}{suffix}");
        prop_assert!(validate_identifier("test_field", &invalid, 32).is_err());
    }

    #[test]
    fn validate_email_never_panics_on_arbitrary_input(s in ".*") {
        let _ = validate_email(&s);
    }

    #[test]
    fn validate_money_never_panics_on_arbitrary_input(
        amount in any::<i64>(),
        currency in ".*"
    ) {
        let _ = validate_money(amount, &currency);
    }

    #[test]
    fn validate_money_accepts_valid_amount_and_currency(
        amount in 0i64..1_000_000_000,
        currency in "[a-zA-Z]{3}"
    ) {
        prop_assert!(validate_money(amount, &currency).is_ok());
    }

    #[test]
    fn validate_money_rejects_negative_or_invalid_currency(
        neg_amount in -1_000_000i64..-1i64,
        valid_amount in 0i64..1000,
        bad_currency in "[0-9!@#]{3}|[a-zA-Z]{1,2}|[a-zA-Z]{4,8}"
    ) {
        prop_assert!(validate_money(neg_amount, "EUR").is_err());
        prop_assert!(validate_money(valid_amount, &bad_currency).is_err());
    }

    #[test]
    fn validate_https_url_never_panics_on_arbitrary_input(s in ".*") {
        let _ = validate_https_url("test_url", &s);
    }

    #[test]
    fn validate_https_url_accepts_valid_https(
        host in "[a-z0-9]([a-z0-9\\-]{1,14}[a-z0-9])?\\.[a-z]{2,6}",
        path in "/[a-z0-9/_\\-]{0,32}"
    ) {
        let url = format!("https://{host}{path}");
        prop_assert!(validate_https_url("test_url", &url).is_ok());
    }

    #[test]
    fn validate_https_url_rejects_plain_http_remote(
        host in "[a-z0-9]([a-z0-9\\-]{1,14}[a-z0-9])?\\.[a-z]{2,6}",
        path in "/[a-z0-9/_\\-]{0,32}"
    ) {
        let url = format!("http://{host}{path}");
        prop_assert!(validate_https_url("test_url", &url).is_err());
    }

    #[test]
    fn validate_text_never_panics_and_rejects_newlines(
        prefix in "[a-zA-Z0-9 ]{0,16}",
        newline in "[\r\n]{1,2}",
        suffix in "[a-zA-Z0-9 ]{0,16}"
    ) {
        let with_newline = format!("{prefix}{newline}{suffix}");
        prop_assert!(validate_text("test_text", &with_newline).is_err());
    }
}
