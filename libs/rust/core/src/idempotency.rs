use axum::http::{HeaderMap, Method, Uri};
use sha2::{Digest, Sha256};
const IDEMPOTENCY_KEY_MAX_LEN: usize = 255;

#[derive(Debug, Clone)]
pub struct IdempotencyScope(pub String);

impl IdempotencyScope {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for IdempotencyScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

pub fn validate_key(key: &str) -> Result<&str, String> {
    let trimmed = key.trim();
    if trimmed.is_empty() {
        return Err("Idempotency-Key must not be empty".to_string());
    }
    if trimmed.len() > IDEMPOTENCY_KEY_MAX_LEN {
        return Err(format!(
            "Idempotency-Key must not exceed {IDEMPOTENCY_KEY_MAX_LEN} characters"
        ));
    }
    Ok(trimmed)
}

pub fn derive_scope(headers: &HeaderMap, method: &Method, uri: &Uri) -> IdempotencyScope {
    let mut hasher = Sha256::new();

    if let Some(auth) = headers.get("Authorization").and_then(|v| v.to_str().ok()) {
        hasher.update(b"auth:");
        hasher.update(auth.as_bytes());
    } else if let Some(ip) = headers
        .get("x-nvbes-client-ip")
        .and_then(|v| v.to_str().ok())
    {
        hasher.update(b"ip:");
        hasher.update(ip.as_bytes());
    } else {
        hasher.update(b"anon:");
    }

    hasher.update(b":");
    hasher.update(method.as_str().as_bytes());
    hasher.update(b":");
    hasher.update(uri.path().as_bytes());

    let hash = data_encoding::HEXLOWER.encode(&hasher.finalize());
    IdempotencyScope(hash)
}

pub fn hash_request_body(body: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(body);
    data_encoding::HEXLOWER.encode(&hasher.finalize())
}

pub fn build_request_signature(method: &Method, uri: &Uri, body: &[u8]) -> String {
    let body_hash = hash_request_body(body);
    format!("{}:{}:{}", method.as_str(), uri.path(), body_hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_key_trims_and_rejects_invalid_values() {
        assert_eq!(
            validate_key(" key-123 ").expect("key should be valid"),
            "key-123"
        );
        assert!(validate_key("   ").is_err());
        assert!(validate_key(&"a".repeat(IDEMPOTENCY_KEY_MAX_LEN + 1)).is_err());
    }

    #[test]
    fn request_signature_includes_method_path_and_body_hash() {
        let method = Method::POST;
        let uri = "/v1/workspaces/abc/objects?ignored=true"
            .parse::<Uri>()
            .expect("uri must parse");

        let signature = build_request_signature(&method, &uri, br#"{"name":"file"}"#);

        assert!(signature.starts_with("POST:/v1/workspaces/abc/objects:"));
        assert_eq!(
            signature.len(),
            "POST:/v1/workspaces/abc/objects:".len() + 64
        );
    }

    #[test]
    fn derive_scope_changes_by_actor_and_route() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "Authorization",
            "Bearer token-a".parse().expect("valid header"),
        );
        let method = Method::POST;
        let first_uri = "/v1/workspaces/a/objects"
            .parse::<Uri>()
            .expect("uri must parse");
        let second_uri = "/v1/workspaces/b/objects"
            .parse::<Uri>()
            .expect("uri must parse");

        let first = derive_scope(&headers, &method, &first_uri);
        let second = derive_scope(&headers, &method, &second_uri);

        assert_ne!(first.as_str(), second.as_str());
        assert_eq!(first.as_str().len(), 64);
        assert_eq!(format!("{first}").len(), 64);
    }

    #[test]
    fn derive_scope_falls_back_to_client_ip_then_anonymous() {
        let method = Method::POST;
        let uri = "/v1/resources".parse::<Uri>().expect("uri");

        let mut with_ip = HeaderMap::new();
        with_ip.insert(
            "x-nvbes-client-ip",
            "203.0.113.10".parse().expect("valid header"),
        );
        let ip_scope = derive_scope(&with_ip, &method, &uri);

        let anon_scope = derive_scope(&HeaderMap::new(), &method, &uri);
        assert_ne!(ip_scope.as_str(), anon_scope.as_str());
        assert_eq!(ip_scope.as_str().len(), 64);
        assert_eq!(anon_scope.as_str().len(), 64);
    }
}
