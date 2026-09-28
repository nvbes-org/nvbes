use std::collections::HashMap;

use tonic::{Request, Status, metadata::MetadataMap};

pub fn authenticate_producer<T>(
    request: &Request<T>,
    producer_tokens: &HashMap<String, String>,
    producer: &str,
) -> Result<(), Status> {
    let expected_token = producer_tokens
        .get(producer)
        .ok_or_else(|| Status::unauthenticated("internal authentication required"))?;
    authenticate_metadata(request.metadata(), expected_token)
}

fn authenticate_metadata(metadata: &MetadataMap, expected_token: &str) -> Result<(), Status> {
    let provided = metadata
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .unwrap_or_default();
    if constant_time_eq(provided.as_bytes(), expected_token.as_bytes()) {
        Ok(())
    } else {
        Err(Status::unauthenticated("internal authentication required"))
    }
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0_u8, |difference, (left, right)| {
            difference | (left ^ right)
        })
        == 0
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use tonic::Request;

    use super::authenticate_producer;

    const TOKEN: &str = "email-worker-internal-token-32-value";

    #[test]
    fn bearer_authentication_requires_an_exact_match() {
        let mut request = Request::new(());
        request
            .metadata_mut()
            .insert("authorization", format!("Bearer {TOKEN}").parse().unwrap());

        // Same length as TOKEN so length-mismatch short-circuit cannot hide
        // a broken constant-time accumulator (e.g. `|` mutated to `&`).
        const OTHER_TOKEN: &str = "email-worker-internal-token-xx-value";
        assert_eq!(TOKEN.len(), OTHER_TOKEN.len());

        let tokens = HashMap::from([
            ("identity-service".to_string(), TOKEN.to_string()),
            ("billing-worker".to_string(), OTHER_TOKEN.to_string()),
        ]);
        assert!(authenticate_producer(&request, &tokens, "identity-service").is_ok());
        assert!(authenticate_producer(&request, &tokens, "billing-worker").is_err());
        assert!(authenticate_producer(&request, &tokens, "unknown-service").is_err());
    }

    #[test]
    fn constant_time_eq_rejects_same_length_mismatches() {
        assert!(super::constant_time_eq(b"abcd", b"abcd"));
        assert!(!super::constant_time_eq(b"abcd", b"abce"));
        assert!(!super::constant_time_eq(b"abcd", b"abc"));
        assert!(!super::constant_time_eq(b"ab", b"abcd"));
        // Two differing bytes that cancel under XOR must still reject (kills `|`→`^`).
        assert!(!super::constant_time_eq(&[1, 1], &[0, 0]));
    }
}
