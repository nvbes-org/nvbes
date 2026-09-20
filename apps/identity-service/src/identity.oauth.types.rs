use serde::{Deserialize, Serialize};

pub fn validate_client_id(client_id: &str) -> bool {
    // Basic validation: client_id should be alphanumeric with hyphens/underscores
    !client_id.is_empty()
        && client_id.len() <= 128
        && client_id
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
}

pub fn validate_redirect_uri(uri: &str) -> bool {
    // Basic validation: should be a valid URL with https (or http for localhost in dev)
    if uri.is_empty() || uri.len() > 2048 {
        return false;
    }
    // In development, allow http; in production, require https
    // For now, just check it starts with http:// or https://
    uri.starts_with("http://") || uri.starts_with("https://")
}

pub fn validate_scope(scope: &str) -> bool {
    // Validate scope format: space-separated tokens
    if scope.is_empty() {
        return true; // Empty scope is valid
    }
    scope.split_whitespace().all(|token| {
        !token.is_empty()
            && token.len() <= 64
            && token
                .chars()
                .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == ':')
    })
}

#[derive(Debug, Deserialize)]
pub struct TokenRequest {
    pub grant_type: String,
    pub code: Option<String>,
    pub redirect_uri: Option<String>,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub refresh_token: Option<String>,
    pub scope: Option<String>,
    pub code_verifier: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
    pub scope: String,
    pub refresh_token: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TokenErrorResponse {
    pub error: String,
    pub error_description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AuthorizeRequest {
    pub response_type: String,
    pub client_id: String,
    pub redirect_uri: String,
    pub scope: Option<String>,
    pub state: Option<String>,
    pub code_challenge: Option<String>,
    pub code_challenge_method: Option<String>,
}
