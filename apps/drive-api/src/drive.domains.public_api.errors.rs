use axum::http::StatusCode;

use crate::http::error::AppError;

#[derive(Clone, Copy)]
pub enum PublicApiErrorKind {
    HttpSignatureNotEnabled,
    HttpSignatureRequired,
    HttpSignatureInvalid,
    HttpSignatureExpired,
    HttpSignatureFuture,
    HttpSignatureInputInvalid,
    HttpSignatureKeyInvalid,
    InvalidApiKey,
    RevokedApiKey,
    ExpiredApiKey,
    WorkspaceInactive,
    InsufficientApiKeyScope,
    InsufficientM2mScope,
    WorkspaceRoleRequired,
    InvalidTokenMissingWorkspaceId,
    InvalidTokenInvalidSub,
    InvalidWorkspace,
    ApiKeyNonceReplayed,
    ApiKeySignatureInvalid,
    ApiKeyTimestampInvalidFormat,
    ApiKeyTimestampInvalid,
    ApiKeySignatureExpired,
    ApiKeySignatureFuture,
    ApiKeyNonceInvalid,
    WorkspaceScopeMismatch,
    NetworkRiskBlocked,
    StepUpNotSupported,
    PermissionDenied,
}

impl PublicApiErrorKind {
    pub fn code(self) -> &'static str {
        match self {
            Self::HttpSignatureNotEnabled => "http_signature_not_enabled",
            Self::HttpSignatureRequired => "http_signature_required",
            Self::HttpSignatureInvalid => "http_signature_invalid",
            Self::HttpSignatureExpired => "http_signature_expired",
            Self::HttpSignatureFuture => "http_signature_future",
            Self::HttpSignatureInputInvalid => "http_signature_input_invalid",
            Self::HttpSignatureKeyInvalid => "http_signature_key_invalid",
            Self::InvalidApiKey => "invalid_api_key",
            Self::RevokedApiKey => "revoked_api_key",
            Self::ExpiredApiKey => "expired_api_key",
            Self::WorkspaceInactive => "workspace_suspended",
            Self::InsufficientApiKeyScope | Self::InsufficientM2mScope => "insufficient_scope",
            Self::WorkspaceRoleRequired => "workspace_role_required",
            Self::InvalidTokenMissingWorkspaceId | Self::InvalidTokenInvalidSub => "invalid_token",
            Self::InvalidWorkspace => "invalid_workspace",
            Self::ApiKeyNonceReplayed => "api_key_nonce_replayed",
            Self::ApiKeySignatureInvalid => "api_key_signature_invalid",
            Self::ApiKeyTimestampInvalidFormat | Self::ApiKeyTimestampInvalid => {
                "api_key_timestamp_invalid"
            }
            Self::ApiKeySignatureExpired => "api_key_signature_expired",
            Self::ApiKeySignatureFuture => "api_key_signature_future",
            Self::ApiKeyNonceInvalid => "api_key_nonce_invalid",
            Self::WorkspaceScopeMismatch => "workspace_suspended",
            Self::NetworkRiskBlocked => "network_risk_blocked",
            Self::StepUpNotSupported => "step_up_not_supported_for_public_api",
            Self::PermissionDenied => "permission_denied",
        }
    }

    pub fn message(self) -> &'static str {
        match self {
            Self::HttpSignatureNotEnabled => "HTTP signatures are not enabled for this API key.",
            Self::HttpSignatureRequired => "This API key requires HTTP Message Signatures.",
            Self::HttpSignatureInvalid => "HTTP signature is invalid.",
            Self::HttpSignatureExpired => "HTTP signature has expired.",
            Self::HttpSignatureFuture => "HTTP signature was created in the future.",
            Self::HttpSignatureInputInvalid => "Signature-Input header is invalid.",
            Self::HttpSignatureKeyInvalid => "HTTP signature public key is invalid.",
            Self::InvalidApiKey => "Invalid API key or M2M token.",
            Self::RevokedApiKey => "API key has been revoked.",
            Self::ExpiredApiKey => "API key has expired.",
            Self::WorkspaceInactive => "Workspace is not active.",
            Self::InsufficientApiKeyScope => "API key does not include the required scope.",
            Self::InsufficientM2mScope => "M2M token does not include the required scope.",
            Self::WorkspaceRoleRequired => "M2M token does not expose a workspace role.",
            Self::InvalidTokenMissingWorkspaceId => "M2M token is missing workspace_id",
            Self::InvalidTokenInvalidSub => "M2M token has invalid sub",
            Self::InvalidWorkspace => "Workspace not found",
            Self::ApiKeyNonceReplayed => "X-Nonce has already been used for this API key.",
            Self::ApiKeySignatureInvalid => "API key signature is invalid.",
            Self::ApiKeyTimestampInvalidFormat => {
                "X-Timestamp must be a Unix timestamp in seconds."
            }
            Self::ApiKeyTimestampInvalid => "X-Timestamp is invalid.",
            Self::ApiKeySignatureExpired => "X-Timestamp is too old.",
            Self::ApiKeySignatureFuture => "X-Timestamp is in the future.",
            Self::ApiKeyNonceInvalid => "X-Nonce is invalid.",
            Self::WorkspaceScopeMismatch => "API key is not scoped to this workspace.",
            Self::NetworkRiskBlocked => {
                "This Public API request is blocked because the source network is high risk."
            }
            Self::StepUpNotSupported => {
                "Step-up actions cannot be performed with Public API credentials."
            }
            Self::PermissionDenied => {
                "You do not have permission to perform this action in this workspace."
            }
        }
    }

    pub fn status(self) -> StatusCode {
        match self {
            Self::HttpSignatureNotEnabled
            | Self::HttpSignatureRequired
            | Self::HttpSignatureInvalid
            | Self::HttpSignatureExpired
            | Self::HttpSignatureFuture
            | Self::HttpSignatureInputInvalid
            | Self::HttpSignatureKeyInvalid
            | Self::InvalidApiKey
            | Self::RevokedApiKey
            | Self::ExpiredApiKey
            | Self::InvalidTokenMissingWorkspaceId
            | Self::InvalidTokenInvalidSub
            | Self::InvalidWorkspace
            | Self::ApiKeyNonceReplayed
            | Self::ApiKeySignatureInvalid
            | Self::ApiKeyTimestampInvalidFormat
            | Self::ApiKeyTimestampInvalid
            | Self::ApiKeySignatureExpired
            | Self::ApiKeySignatureFuture
            | Self::ApiKeyNonceInvalid => StatusCode::UNAUTHORIZED,
            Self::WorkspaceInactive
            | Self::InsufficientApiKeyScope
            | Self::InsufficientM2mScope
            | Self::WorkspaceRoleRequired
            | Self::WorkspaceScopeMismatch
            | Self::NetworkRiskBlocked
            | Self::StepUpNotSupported
            | Self::PermissionDenied => StatusCode::FORBIDDEN,
        }
    }

    pub fn app_error(self) -> AppError {
        AppError::new(self.status(), self.code(), self.message())
    }

    pub fn header_missing(self, name: &str) -> AppError {
        let message = match self {
            Self::ApiKeySignatureInvalid => {
                format!("API key authentication requires header {name}.")
            }
            Self::HttpSignatureInvalid => format!("HTTP signature requires header {name}."),
            _ => self.message().to_string(),
        };
        AppError::new(self.status(), self.code(), message)
    }

    pub fn header_invalid(self, name: &str) -> AppError {
        let message = match self {
            Self::ApiKeySignatureInvalid => {
                format!("API key authentication header {name} is invalid.")
            }
            Self::HttpSignatureInvalid => {
                format!("HTTP signature header {name} is invalid.")
            }
            _ => self.message().to_string(),
        };
        AppError::new(self.status(), self.code(), message)
    }

    pub fn component_missing(self, component: &str) -> AppError {
        let message = match self {
            Self::HttpSignatureInvalid => {
                format!("HTTP signature must cover {component}.")
            }
            _ => self.message().to_string(),
        };
        AppError::new(self.status(), self.code(), message)
    }
}
