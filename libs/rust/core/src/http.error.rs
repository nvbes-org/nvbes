use crate::limiter::RateLimitInfo;
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorEnvelope {
    pub error: ErrorBody,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorBody {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recovery: Option<ErrorRecovery>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ErrorRecovery {
    Reauthenticate,
}

pub fn public_error_message(status: StatusCode, message: String) -> String {
    if status == StatusCode::INTERNAL_SERVER_ERROR {
        "Internal server error.".to_string()
    } else {
        message
    }
}

pub fn public_error_code(status: StatusCode, code: String) -> String {
    if status == StatusCode::INTERNAL_SERVER_ERROR {
        "internal_error".to_string()
    } else {
        code
    }
}

#[derive(Debug, Clone)]
pub struct AppError {
    pub status: StatusCode,
    pub code: String,
    pub message: String,
    pub recovery: Option<ErrorRecovery>,
    pub retry_after_seconds: Option<u64>,
    pub request_id: Option<String>,
    pub rate_limit_info: Option<Box<RateLimitInfo>>,
}

impl AppError {
    pub fn new(status: StatusCode, code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            status,
            code: code.into(),
            message: message.into(),
            recovery: None,
            retry_after_seconds: None,
            request_id: None,
            rate_limit_info: None,
        }
    }

    pub fn bad_request(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, code, message)
    }

    pub fn unauthorized(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, code, message)
    }

    pub fn reauthenticate(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::unauthorized(code, message).requiring_reauthentication()
    }

    pub fn requiring_reauthentication(mut self) -> Self {
        self.recovery = Some(ErrorRecovery::Reauthenticate);
        self
    }

    pub fn forbidden(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(StatusCode::FORBIDDEN, code, message)
    }

    pub fn not_found(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, code, message)
    }

    pub fn conflict(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(StatusCode::CONFLICT, code, message)
    }

    pub fn precondition_failed(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(StatusCode::PRECONDITION_FAILED, code, message)
    }

    pub fn too_many_requests(
        code: impl Into<String>,
        message: impl Into<String>,
        retry_after_seconds: Option<u64>,
        rate_limit_info: Option<RateLimitInfo>,
    ) -> Self {
        Self {
            status: StatusCode::TOO_MANY_REQUESTS,
            code: code.into(),
            message: message.into(),
            recovery: None,
            retry_after_seconds,
            request_id: None,
            rate_limit_info: rate_limit_info.map(Box::new),
        }
    }

    pub fn internal(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, code, message)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        use axum::http::header;
        let status = self.status;
        let code = self.code;
        let recovery = self.recovery;
        let request_id = self.request_id;
        if status == StatusCode::INTERNAL_SERVER_ERROR {
            tracing::error!(
                error.code = %code,
                error.message = %self.message,
                error.request_id = request_id.as_deref().unwrap_or(""),
                "internal API error"
            );
        }
        let code = public_error_code(status, code);
        let message = public_error_message(status, self.message);
        let mut response = (
            status,
            Json(ErrorEnvelope {
                error: ErrorBody {
                    code,
                    message,
                    recovery,
                    request_id,
                },
            }),
        )
            .into_response();
        if status == StatusCode::TOO_MANY_REQUESTS {
            if let Some(seconds) = self.retry_after_seconds {
                response
                    .headers_mut()
                    .insert(header::RETRY_AFTER, seconds.to_string().parse().unwrap());
            }
            if let Some(ref info) = self.rate_limit_info {
                info.append_headers(response.headers_mut());
            }
        }
        response
    }
}

#[macro_export]
macro_rules! impl_app_error {
    () => {
        use axum::{
            Json,
            http::StatusCode,
            response::{IntoResponse, Response},
        };
        use $crate::http::error::{ErrorBody, ErrorEnvelope, ErrorRecovery};
        use $crate::limiter::RateLimitInfo;

        #[derive(Debug, Clone)]
        pub struct AppError {
            pub status: StatusCode,
            pub code: String,
            pub message: String,
            pub recovery: Option<ErrorRecovery>,
            pub retry_after_seconds: Option<u64>,
            pub request_id: Option<String>,
            pub rate_limit_info: Option<Box<RateLimitInfo>>,
        }

        impl AppError {
            pub fn new(
                status: StatusCode,
                code: impl Into<String>,
                message: impl Into<String>,
            ) -> Self {
                Self {
                    status,
                    code: code.into(),
                    message: message.into(),
                    recovery: None,
                    retry_after_seconds: None,
                    request_id: None,
                    rate_limit_info: None,
                }
            }

            pub fn bad_request(code: impl Into<String>, message: impl Into<String>) -> Self {
                Self::new(StatusCode::BAD_REQUEST, code, message)
            }

            pub fn unauthorized(code: impl Into<String>, message: impl Into<String>) -> Self {
                Self::new(StatusCode::UNAUTHORIZED, code, message)
            }

            pub fn reauthenticate(code: impl Into<String>, message: impl Into<String>) -> Self {
                Self::unauthorized(code, message).requiring_reauthentication()
            }

            pub fn requiring_reauthentication(mut self) -> Self {
                self.recovery = Some(ErrorRecovery::Reauthenticate);
                self
            }

            pub fn forbidden(code: impl Into<String>, message: impl Into<String>) -> Self {
                Self::new(StatusCode::FORBIDDEN, code, message)
            }

            pub fn not_found(code: impl Into<String>, message: impl Into<String>) -> Self {
                Self::new(StatusCode::NOT_FOUND, code, message)
            }

            pub fn conflict(code: impl Into<String>, message: impl Into<String>) -> Self {
                Self::new(StatusCode::CONFLICT, code, message)
            }

            pub fn precondition_failed(
                code: impl Into<String>,
                message: impl Into<String>,
            ) -> Self {
                Self::new(StatusCode::PRECONDITION_FAILED, code, message)
            }

            pub fn too_many_requests(
                code: impl Into<String>,
                message: impl Into<String>,
                retry_after_seconds: Option<u64>,
                rate_limit_info: Option<RateLimitInfo>,
            ) -> Self {
                Self {
                    status: StatusCode::TOO_MANY_REQUESTS,
                    code: code.into(),
                    message: message.into(),
                    recovery: None,
                    retry_after_seconds,
                    request_id: None,
                    rate_limit_info: rate_limit_info.map(Box::new),
                }
            }

            pub fn internal(code: impl Into<String>, message: impl Into<String>) -> Self {
                Self::new(StatusCode::INTERNAL_SERVER_ERROR, code, message)
            }
        }

        impl IntoResponse for AppError {
            fn into_response(self) -> Response {
                use axum::http::header;
                let status = self.status;
                let code = self.code;
                let recovery = self.recovery;
                let request_id = self.request_id;
                if status == StatusCode::INTERNAL_SERVER_ERROR {
                    tracing::error!(
                        error.code = %code,
                        error.message = %self.message,
                        error.request_id = request_id.as_deref().unwrap_or(""),
                        "internal API error"
                    );
                }
                let code = $crate::http::error::public_error_code(status, code);
                let message = $crate::http::error::public_error_message(status, self.message);
                let mut response = (
                    status,
                    Json(ErrorEnvelope {
                        error: ErrorBody {
                            code,
                            message,
                            recovery,
                            request_id,
                        },
                    }),
                )
                    .into_response();
                if status == StatusCode::TOO_MANY_REQUESTS {
                    if let Some(seconds) = self.retry_after_seconds {
                        response
                            .headers_mut()
                            .insert(header::RETRY_AFTER, seconds.to_string().parse().unwrap());
                    }
                    if let Some(ref info) = self.rate_limit_info {
                        info.append_headers(response.headers_mut());
                    }
                }
                response
            }
        }

        impl From<$crate::http::error::AppError> for AppError {
            fn from(err: $crate::http::error::AppError) -> Self {
                Self {
                    status: err.status,
                    code: err.code,
                    message: err.message,
                    recovery: err.recovery,
                    retry_after_seconds: err.retry_after_seconds,
                    request_id: err.request_id,
                    rate_limit_info: err.rate_limit_info,
                }
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::{AppError, public_error_code, public_error_message};
    use axum::http::StatusCode;

    #[test]
    fn app_error_constructors_keep_standard_error_contract() {
        let error = AppError::bad_request("invalid_input", "Invalid input.");

        assert_eq!(error.status, StatusCode::BAD_REQUEST);
        assert_eq!(error.code, "invalid_input");
        assert_eq!(error.message, "Invalid input.");
        assert!(error.request_id.is_none());
    }

    #[test]
    fn reauthentication_error_explicitly_requests_reauthentication() {
        let error = AppError::reauthenticate("session_expired", "Session expired.");

        assert_eq!(error.status, StatusCode::UNAUTHORIZED);
        assert_eq!(error.recovery, Some(super::ErrorRecovery::Reauthenticate));
    }

    #[test]
    fn rate_limit_error_preserves_retry_after() {
        let error = AppError::too_many_requests("rate_limited", "Slow down.", Some(30), None);

        assert_eq!(error.status, StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(error.retry_after_seconds, Some(30));
    }

    #[test]
    fn public_error_message_hides_internal_details() {
        let message = public_error_message(
            StatusCode::INTERNAL_SERVER_ERROR,
            "database constraint geo_lookup_events_purpose_check failed".to_string(),
        );

        assert_eq!(message, "Internal server error.");
    }

    #[test]
    fn public_error_message_preserves_client_errors() {
        let message = public_error_message(StatusCode::BAD_REQUEST, "Invalid email.".to_string());

        assert_eq!(message, "Invalid email.");
    }

    #[test]
    fn public_error_code_hides_internal_codes() {
        let code = public_error_code(
            StatusCode::INTERNAL_SERVER_ERROR,
            "database_error".to_string(),
        );

        assert_eq!(code, "internal_error");
    }

    #[test]
    fn public_error_code_preserves_client_error_codes() {
        let code = public_error_code(StatusCode::BAD_REQUEST, "invalid_email".to_string());

        assert_eq!(code, "invalid_email");
    }
}
