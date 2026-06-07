use axum::http::{HeaderMap, Method, Uri};
use chrono::{TimeZone, Utc};
use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::domains::public_api::errors::PublicApiErrorKind;
use crate::http::error::AppError;

const X_NONCE: &str = "x-nonce";
const X_TIMESTAMP: &str = "x-timestamp";
const X_SIGNATURE: &str = "x-signature";
const MAX_CLOCK_SKEW_SECS: i64 = 300;
const MIN_NONCE_LEN: usize = 16;
const MAX_NONCE_LEN: usize = 128;

type HmacSha256 = Hmac<Sha256>;

pub struct VerifiedApiKeySignature {
    pub nonce: String,
    pub timestamp: chrono::DateTime<Utc>,
}

pub fn verify(
    headers: &HeaderMap,
    method: &Method,
    uri: &Uri,
    api_key: &str,
) -> Result<VerifiedApiKeySignature, AppError> {
    let nonce = required_header(headers, X_NONCE)?;
    validate_nonce(nonce)?;

    let timestamp = parse_timestamp(required_header(headers, X_TIMESTAMP)?)?;
    validate_timestamp(timestamp)?;

    let provided_signature = required_header(headers, X_SIGNATURE)?;
    let expected_signature = signature(api_key, method, uri, timestamp.timestamp(), nonce)?;
    if !constant_time_eq(provided_signature.as_bytes(), expected_signature.as_bytes()) {
        return Err(PublicApiErrorKind::ApiKeySignatureInvalid.app_error());
    }

    Ok(VerifiedApiKeySignature {
        nonce: nonce.to_owned(),
        timestamp,
    })
}

fn signature(
    api_key: &str,
    method: &Method,
    uri: &Uri,
    timestamp: i64,
    nonce: &str,
) -> Result<String, AppError> {
    let message = format!(
        "{}\n{}\n{}\n{}",
        method.as_str().to_ascii_uppercase(),
        uri.path_and_query()
            .map(|value| value.as_str())
            .unwrap_or("/"),
        timestamp,
        nonce
    );
    let mut mac = HmacSha256::new_from_slice(api_key.as_bytes())
        .map_err(|_| PublicApiErrorKind::ApiKeySignatureInvalid.app_error())?;
    mac.update(message.as_bytes());
    Ok(nvbes_billing::hex_encode(&mac.finalize().into_bytes()))
}

fn parse_timestamp(value: &str) -> Result<chrono::DateTime<Utc>, AppError> {
    let timestamp = value
        .parse::<i64>()
        .map_err(|_| PublicApiErrorKind::ApiKeyTimestampInvalidFormat.app_error())?;
    Utc.timestamp_opt(timestamp, 0)
        .single()
        .ok_or_else(|| PublicApiErrorKind::ApiKeyTimestampInvalid.app_error())
}

fn validate_timestamp(timestamp: chrono::DateTime<Utc>) -> Result<(), AppError> {
    let now = Utc::now().timestamp();
    let timestamp = timestamp.timestamp();
    if timestamp < now - MAX_CLOCK_SKEW_SECS {
        return Err(PublicApiErrorKind::ApiKeySignatureExpired.app_error());
    }
    if timestamp > now + MAX_CLOCK_SKEW_SECS {
        return Err(PublicApiErrorKind::ApiKeySignatureFuture.app_error());
    }
    Ok(())
}

fn validate_nonce(nonce: &str) -> Result<(), AppError> {
    let valid_len = (MIN_NONCE_LEN..=MAX_NONCE_LEN).contains(&nonce.len());
    let valid_chars = nonce
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'));
    if valid_len && valid_chars {
        return Ok(());
    }
    Err(PublicApiErrorKind::ApiKeyNonceInvalid.app_error())
}

fn required_header<'a>(headers: &'a HeaderMap, name: &str) -> Result<&'a str, AppError> {
    headers
        .get(name)
        .ok_or_else(|| PublicApiErrorKind::ApiKeySignatureInvalid.header_missing(name))?
        .to_str()
        .map_err(|_| PublicApiErrorKind::ApiKeySignatureInvalid.header_invalid(name))
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right.iter())
        .fold(0_u8, |acc, (left, right)| acc | (left ^ right))
        == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn verify_accepts_valid_hmac_signature() {
        let api_key = "gx_test_secret";
        let method = Method::GET;
        let uri = Uri::from_static("/v1/workspaces/ws/objects?parent_id=root");
        let nonce = "nonce_1234567890";
        let timestamp = Utc::now().timestamp();

        let mut headers = HeaderMap::new();
        headers.insert(X_NONCE, HeaderValue::from_static(nonce));
        headers.insert(
            X_TIMESTAMP,
            HeaderValue::from_str(&timestamp.to_string()).unwrap(),
        );
        headers.insert(
            X_SIGNATURE,
            HeaderValue::from_str(&signature(api_key, &method, &uri, timestamp, nonce).unwrap())
                .unwrap(),
        );

        let verified = verify(&headers, &method, &uri, api_key).unwrap();
        assert_eq!(verified.nonce, nonce);
    }

    #[test]
    fn verify_rejects_tampered_path() {
        let api_key = "gx_test_secret";
        let method = Method::GET;
        let signed_uri = Uri::from_static("/v1/workspaces/ws/objects");
        let checked_uri = Uri::from_static("/v1/workspaces/ws/objects?parent_id=root");
        let nonce = "nonce_1234567890";
        let timestamp = Utc::now().timestamp();

        let mut headers = HeaderMap::new();
        headers.insert(X_NONCE, HeaderValue::from_static(nonce));
        headers.insert(
            X_TIMESTAMP,
            HeaderValue::from_str(&timestamp.to_string()).unwrap(),
        );
        headers.insert(
            X_SIGNATURE,
            HeaderValue::from_str(
                &signature(api_key, &method, &signed_uri, timestamp, nonce).unwrap(),
            )
            .unwrap(),
        );

        assert!(verify(&headers, &method, &checked_uri, api_key).is_err());
    }
}
