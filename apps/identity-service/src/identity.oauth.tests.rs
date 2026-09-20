#[cfg(test)]
mod tests {
    use crate::oauth::validate_client_id;
    use crate::oauth::validate_redirect_uri;

    fn validate_scope(scope: &str) -> bool {
        if scope.is_empty() {
            return true;
        }
        scope
            .split_whitespace()
            .all(|token| {
                !token.is_empty()
                    && token.len() <= 64
                    && token
                        .chars()
                        .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == ':')
            })
    }

    #[test]
    fn validate_client_id_accepts_valid_formats() {
        assert!(validate_client_id("my-app"));
        assert!(validate_client_id("my_app_123"));
        assert!(validate_client_id("test-client-v1"));
        assert!(validate_client_id("a"));
        assert!(validate_client_id("client_with_underscores_and-hyphens"));
    }

    #[test]
    fn validate_client_id_rejects_invalid_formats() {
        assert!(!validate_client_id(""));
        assert!(!validate_client_id("client with spaces"));
        assert!(!validate_client_id("client@domain"));
        assert!(!validate_client_id("client!exclamation"));
        assert!(!validate_client_id("client.dots"));
    }

    #[test]
    fn validate_redirect_uri_accepts_valid_urls() {
        assert!(validate_redirect_uri("https://example.com/callback"));
        assert!(validate_redirect_uri("http://localhost:3000/callback"));
        assert!(validate_redirect_uri("https://app.example.com/auth/callback"));
    }

    #[test]
    fn validate_redirect_uri_rejects_invalid_urls() {
        assert!(!validate_redirect_uri(""));
        assert!(!validate_redirect_uri("not-a-url"));
        assert!(!validate_redirect_uri("ftp://example.com/callback"));
    }

    #[test]
    fn validate_scope_accepts_valid_scopes() {
        assert!(validate_scope("openid"));
        assert!(validate_scope("openid profile email"));
        assert!(validate_scope("account:read account:write"));
        assert!(validate_scope(""));
    }

    #[test]
    fn validate_scope_rejects_invalid_scopes() {
        assert!(!validate_scope("scope@invalid"));
        assert!(!validate_scope("scope!exclamation"));
    }
}
