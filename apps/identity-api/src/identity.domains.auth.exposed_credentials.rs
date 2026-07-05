use axum::http::HeaderMap;

use crate::http::error::AppError;

const EXPOSED_CREDENTIAL_CHECK_HEADER: &str = "Exposed-Credential-Check";
const USERNAME_PASSWORD_PAIR: u8 = 1;
const USERNAME_ONLY: u8 = 2;
const PASSWORD_ONLY: u8 = 4;
const KNOWN_BITS: u8 = USERNAME_PASSWORD_PAIR | USERNAME_ONLY | PASSWORD_ONLY;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExposedCredentialCheck {
    bits: u8,
}

impl ExposedCredentialCheck {
    pub fn from_headers(headers: &HeaderMap) -> Option<Self> {
        if !nvbes_core::http::client_ip::from_trusted_proxy(headers) {
            return None;
        }

        let value = headers
            .get(EXPOSED_CREDENTIAL_CHECK_HEADER)
            .and_then(|value| value.to_str().ok())?;

        Self::parse(value)
    }

    pub fn parse(value: &str) -> Option<Self> {
        let bits = value
            .split(',')
            .filter_map(|part| part.trim().parse::<u8>().ok())
            .fold(0, |acc, value| acc | (value & KNOWN_BITS));

        (bits != 0).then_some(Self { bits })
    }

    pub fn password_leaked(self) -> bool {
        self.contains(USERNAME_PASSWORD_PAIR) || self.contains(PASSWORD_ONLY)
    }

    pub fn username_leaked(self) -> bool {
        self.contains(USERNAME_PASSWORD_PAIR) || self.contains(USERNAME_ONLY)
    }

    pub fn username_password_pair_leaked(self) -> bool {
        self.contains(USERNAME_PASSWORD_PAIR)
    }

    pub fn labels(self) -> Vec<&'static str> {
        let mut labels = Vec::new();
        if self.username_password_pair_leaked() {
            labels.push("username_password_pair");
        }
        if self.contains(USERNAME_ONLY) {
            labels.push("username");
        }
        if self.contains(PASSWORD_ONLY) {
            labels.push("password");
        }
        labels
    }

    fn contains(self, bit: u8) -> bool {
        self.bits & bit != 0
    }
}

pub fn check_new_password(headers: &HeaderMap) -> Result<Option<ExposedCredentialCheck>, AppError> {
    let Some(check) = ExposedCredentialCheck::from_headers(headers) else {
        return Ok(None);
    };

    if check.password_leaked() {
        return Err(AppError::bad_request(
            "password_exposed",
            "This password has appeared in a public data breach. Choose a different password.",
        ));
    }

    Ok(Some(check))
}

pub fn login_rejected_error() -> AppError {
    AppError::forbidden(
        "password_compromised",
        "This password has appeared in a public data breach. Reset it before signing in.",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn parse_supports_cloudflare_bit_values() {
        let check = ExposedCredentialCheck::parse("1, 4").expect("header should parse");

        assert!(check.username_password_pair_leaked());
        assert!(check.username_leaked());
        assert!(check.password_leaked());
        assert_eq!(check.labels(), vec!["username_password_pair", "password"]);
    }

    #[test]
    fn parse_ignores_unknown_values() {
        assert!(ExposedCredentialCheck::parse("0,8,invalid").is_none());
    }

    #[test]
    fn from_headers_requires_trusted_proxy_marker() {
        let mut headers = HeaderMap::new();
        headers.insert(
            EXPOSED_CREDENTIAL_CHECK_HEADER,
            HeaderValue::from_static("4"),
        );

        assert!(ExposedCredentialCheck::from_headers(&headers).is_none());

        headers.insert(
            nvbes_core::http::client_ip::TRUSTED_PROXY_HEADER,
            HeaderValue::from_static("1"),
        );

        assert!(
            ExposedCredentialCheck::from_headers(&headers)
                .expect("trusted header should parse")
                .password_leaked()
        );
    }

    #[test]
    fn check_new_password_rejects_password_leaks() {
        let mut headers = HeaderMap::new();
        headers.insert(
            nvbes_core::http::client_ip::TRUSTED_PROXY_HEADER,
            HeaderValue::from_static("1"),
        );
        headers.insert(
            EXPOSED_CREDENTIAL_CHECK_HEADER,
            HeaderValue::from_static("4"),
        );

        let error = check_new_password(&headers).expect_err("leaked password should be rejected");

        assert_eq!(error.code, "password_exposed");
    }

    #[test]
    fn check_new_password_allows_username_only_leaks() {
        let mut headers = HeaderMap::new();
        headers.insert(
            nvbes_core::http::client_ip::TRUSTED_PROXY_HEADER,
            HeaderValue::from_static("1"),
        );
        headers.insert(
            EXPOSED_CREDENTIAL_CHECK_HEADER,
            HeaderValue::from_static("2"),
        );

        let check = check_new_password(&headers)
            .expect("username-only leak should not reject password creation")
            .expect("header should parse");

        assert!(check.username_leaked());
        assert!(!check.password_leaked());
    }
}
