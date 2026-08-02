use axum::http::{HeaderMap, header};

const MINIMUM_TOKEN_LENGTH: usize = 32;

pub fn load_token(
    variable: &str,
    environment: &str,
    development_default: &str,
) -> Result<String, String> {
    debug_assert!(development_default.len() >= MINIMUM_TOKEN_LENGTH);
    match std::env::var(variable) {
        Ok(value) if value.trim().len() >= MINIMUM_TOKEN_LENGTH => Ok(value.trim().to_string()),
        Ok(_) => Err(format!(
            "{variable} must contain at least {MINIMUM_TOKEN_LENGTH} characters"
        )),
        Err(std::env::VarError::NotPresent) if matches!(environment, "development" | "test") => {
            Ok(development_default.to_string())
        }
        Err(std::env::VarError::NotPresent) => Err(format!("{variable} is required")),
        Err(error) => Err(format!("{variable} could not be read: {error}")),
    }
}

pub fn bearer_matches(headers: &HeaderMap, expected: &str) -> bool {
    let provided = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .unwrap_or_default();
    constant_time_eq(provided.as_bytes(), expected.as_bytes())
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
    use axum::http::{HeaderMap, HeaderValue, header};

    use super::bearer_matches;

    #[test]
    fn bearer_authentication_is_exact() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Bearer internal-service-token-value-32"),
        );
        assert!(bearer_matches(&headers, "internal-service-token-value-32"));
        assert!(!bearer_matches(
            &headers,
            "internal-service-token-value-32-extra"
        ));
    }
}
