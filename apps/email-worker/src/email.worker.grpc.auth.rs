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
        request.metadata_mut().insert(
            "authorization",
            "Bearer email-worker-internal-token-32-value"
                .parse()
                .unwrap(),
        );

        let tokens = HashMap::from([
            ("identity-service".to_string(), TOKEN.to_string()),
            (
                "billing-worker".to_string(),
                "different-email-worker-token-32-value".to_string(),
            ),
        ]);
        assert!(authenticate_producer(&request, &tokens, "identity-service").is_ok());
        assert!(authenticate_producer(&request, &tokens, "billing-worker").is_err());
        assert!(authenticate_producer(&request, &tokens, "unknown-service").is_err());
    }
}
