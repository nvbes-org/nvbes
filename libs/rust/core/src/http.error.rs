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
    pub request_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AppError {
    pub status: StatusCode,
    pub code: String,
    pub message: String,
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
        let mut response = (
            self.status,
            Json(ErrorEnvelope {
                error: ErrorBody {
                    code: self.code,
                    message: self.message,
                    request_id: self.request_id,
                },
            }),
        )
            .into_response();
        if self.status == StatusCode::TOO_MANY_REQUESTS {
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
        use $crate::http::error::{ErrorBody, ErrorEnvelope};
        use $crate::limiter::RateLimitInfo;

        #[derive(Debug, Clone)]
        pub struct AppError {
            pub status: StatusCode,
            pub code: String,
            pub message: String,
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
                let mut response = (
                    self.status,
                    Json(ErrorEnvelope {
                        error: ErrorBody {
                            code: self.code,
                            message: self.message,
                            request_id: self.request_id,
                        },
                    }),
                )
                    .into_response();
                if self.status == StatusCode::TOO_MANY_REQUESTS {
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
                    retry_after_seconds: err.retry_after_seconds,
                    request_id: err.request_id,
                    rate_limit_info: err.rate_limit_info,
                }
            }
        }
    };
}
