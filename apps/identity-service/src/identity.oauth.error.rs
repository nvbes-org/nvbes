/// Stable protocol errors; descriptions never include credentials or request input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum OAuthError {
    #[error("invalid_request")]
    InvalidRequest,
    #[error("invalid_client")]
    InvalidClient,
    #[error("invalid_scope")]
    InvalidScope,
    #[error("invalid_target")]
    InvalidTarget,
    #[error("invalid_grant")]
    InvalidGrant,
    #[error("invalid_dpop_proof")]
    InvalidDpopProof,
    #[error("unsupported_response_type")]
    UnsupportedResponseType,
    #[error("access_denied")]
    AccessDenied,
    #[error("login_required")]
    LoginRequired,
    #[error("consent_required")]
    ConsentRequired,
    #[error("temporarily_unavailable")]
    Unavailable,
}
