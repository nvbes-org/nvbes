use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmailFailureClass {
    Transient,
    Ambiguous,
    Permanent,
}

#[derive(Debug, Error)]
pub enum EmailError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("email send failed ({status}): {message}")]
    Api { status: u16, message: String },

    #[error("configuration error: {0}")]
    Config(String),

    #[error("email address error: {0}")]
    Address(#[from] lettre::address::AddressError),

    #[error("email build error: {0}")]
    Build(#[from] lettre::error::Error),

    #[error("smtp error: {0}")]
    Smtp(#[from] lettre::transport::smtp::Error),
}

impl EmailError {
    pub fn failure_class(&self) -> EmailFailureClass {
        use EmailFailureClass::{Ambiguous, Permanent, Transient};

        match self {
            Self::Api { status, .. } => {
                if *status == 408 || *status == 429 || *status >= 500 {
                    Transient
                } else {
                    Permanent
                }
            }
            Self::Http(error) if error.is_timeout() => Ambiguous,
            Self::Http(error) => match error.status() {
                Some(status)
                    if status.is_client_error()
                        && status.as_u16() != 408
                        && status.as_u16() != 429 =>
                {
                    Permanent
                }
                _ => Transient,
            },
            Self::Smtp(error) if error.is_permanent() => Permanent,
            Self::Smtp(_) => Transient,
            Self::Serialization(_) | Self::Config(_) | Self::Address(_) | Self::Build(_) => {
                Permanent
            }
        }
    }

    pub fn safe_code(&self) -> &'static str {
        match self {
            Self::Http(_) => "email_http_transport",
            Self::Serialization(_) => "email_serialization",
            Self::Api { .. } => "email_provider_response",
            Self::Config(_) => "email_configuration",
            Self::Address(_) => "email_address",
            Self::Build(_) => "email_message_build",
            Self::Smtp(_) => "email_smtp_transport",
        }
    }

    pub fn safe_summary(&self) -> &'static str {
        match self {
            Self::Http(_) => "Email provider HTTP request failed",
            Self::Serialization(_) => "Email provider payload could not be serialized",
            Self::Api { .. } => "Email provider rejected the request",
            Self::Config(_) => "Email delivery configuration is invalid",
            Self::Address(_) => "Email address is invalid",
            Self::Build(_) => "Email message could not be built",
            Self::Smtp(_) => "SMTP delivery failed",
        }
    }

    pub fn is_retryable(&self) -> bool {
        matches!(
            self.failure_class(),
            EmailFailureClass::Transient | EmailFailureClass::Ambiguous
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{EmailError, EmailFailureClass};

    #[test]
    fn provider_response_classification_distinguishes_retryable_statuses() {
        for status in [408, 429, 500, 503] {
            let error = EmailError::Api {
                status,
                message: "provider detail must not be persisted".to_string(),
            };
            assert_eq!(error.failure_class(), EmailFailureClass::Transient);
            assert!(error.is_retryable());
        }

        for status in [400, 401, 403, 404, 422] {
            let error = EmailError::Api {
                status,
                message: "provider detail must not be persisted".to_string(),
            };
            assert_eq!(error.failure_class(), EmailFailureClass::Permanent);
            assert!(!error.is_retryable());
        }
    }

    #[test]
    fn safe_metadata_never_contains_provider_details() {
        let secret = "smtp-password=do-not-log";
        let error = EmailError::Api {
            status: 503,
            message: secret.to_string(),
        };

        assert!(!error.safe_code().contains(secret));
        assert!(!error.safe_summary().contains(secret));
    }
}

#[cfg(test)]
#[path = "error.transport.tests.rs"]
mod transport_tests;
