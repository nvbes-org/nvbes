use axum::http::{HeaderName, HeaderValue, Method, header};
use reqwest::Url;
use tower_http::cors::{AllowOrigin, CorsLayer};

/// Explicit browser origins for APIs authenticated by tokens, never cookies.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ResourceCorsOrigins(Vec<HeaderValue>);

#[derive(Debug, thiserror::Error)]
#[error(
    "CORS origins must be a JSON array of at most 32 canonical HTTPS origins; HTTP loopback is allowed only in development"
)]
pub struct InvalidCorsOrigins;

impl ResourceCorsOrigins {
    pub fn from_json(value: &str, development: bool) -> Result<Self, InvalidCorsOrigins> {
        if value.len() > 16_384 {
            return Err(InvalidCorsOrigins);
        }
        let values: Vec<String> = serde_json::from_str(value).map_err(|_| InvalidCorsOrigins)?;
        if values.len() > 32 {
            return Err(InvalidCorsOrigins);
        }
        let mut origins = Vec::with_capacity(values.len());
        for value in values {
            let url = Url::parse(&value).map_err(|_| InvalidCorsOrigins)?;
            let loopback = url.host_str().is_some_and(|host| {
                host == "localhost"
                    || host
                        .trim_matches(['[', ']'])
                        .parse::<std::net::IpAddr>()
                        .is_ok_and(|ip| ip.is_loopback())
            });
            if value != url.origin().ascii_serialization()
                || !(url.scheme() == "https" || (development && url.scheme() == "http" && loopback))
                || url.host_str().is_none_or(|host| host.contains('*'))
            {
                return Err(InvalidCorsOrigins);
            }
            let header = value.parse().map_err(|_| InvalidCorsOrigins)?;
            if !origins.contains(&header) {
                origins.push(header);
            }
        }
        Ok(Self(origins))
    }

    pub fn layer(&self, methods: Vec<Method>) -> CorsLayer {
        CorsLayer::new()
            .allow_origin(AllowOrigin::list(self.0.clone()))
            .allow_methods(methods)
            .allow_headers([
                header::CONTENT_TYPE,
                header::AUTHORIZATION,
                HeaderName::from_static("dpop"),
                HeaderName::from_static("idempotency-key"),
            ])
            .expose_headers([
                header::WWW_AUTHENTICATE,
                header::RETRY_AFTER,
                HeaderName::from_static("dpop-nonce"),
            ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configuration_rejects_ambiguous_or_insecure_origins() {
        for origin in [
            "*",
            "null",
            "https://*.example",
            "https://account.example/",
            "https://account.example/path",
            "https://account.example?query",
            "https://account.example#fragment",
            "https://user@account.example",
            "https://ACCOUNT.example",
            "https://account.example:443",
            "http://account.example",
            "http://localhost",
            "https://account.example\n",
        ] {
            let json = serde_json::to_string(&[origin]).unwrap();
            assert!(
                ResourceCorsOrigins::from_json(&json, false).is_err(),
                "{origin}"
            );
        }
        for origin in [
            "http://localhost:3000",
            "http://127.0.0.1:3000",
            "http://[::1]:3000",
        ] {
            let json = serde_json::to_string(&[origin]).unwrap();
            assert!(ResourceCorsOrigins::from_json(&json, true).is_ok());
        }
        assert!(
            ResourceCorsOrigins::from_json(r#"["http://localhost.attacker.test"]"#, true).is_err()
        );
        assert!(ResourceCorsOrigins::from_json(r#"["https://account.example"]"#, false).is_ok());
        assert!(
            ResourceCorsOrigins::from_json("[]", false)
                .unwrap()
                .0
                .is_empty()
        );
        assert!(ResourceCorsOrigins::from_json("null", false).is_err());
        assert!(ResourceCorsOrigins::from_json(&" ".repeat(16_385), false).is_err());
        let too_many = serde_json::to_string(&vec!["https://account.example"; 33]).unwrap();
        assert!(ResourceCorsOrigins::from_json(&too_many, false).is_err());
    }
}
