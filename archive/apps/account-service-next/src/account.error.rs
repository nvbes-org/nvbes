use axum::{
    Json,
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug)]
pub struct AppError {
    status: StatusCode,
    code: &'static str,
    message: String,
    request_id: Uuid,
    authenticate: Option<String>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ErrorEnvelope {
    pub error: ErrorBody,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ErrorBody {
    pub code: &'static str,
    pub message: String,
    pub request_id: Uuid,
}

impl AppError {
    pub fn bad_request(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, code, message)
    }

    pub fn unauthorized(code: &'static str, message: impl Into<String>) -> Self {
        let mut error = Self::new(StatusCode::UNAUTHORIZED, code, message);
        error.authenticate = Some(format!("Bearer realm=\"nvbes-account\", error=\"{code}\""));
        error
    }

    pub fn insufficient_scope(scope: &'static str) -> Self {
        let mut error = Self::new(
            StatusCode::FORBIDDEN,
            "insufficient_scope",
            format!("The OAuth access token requires the `{scope}` scope."),
        );
        error.authenticate = Some(format!(
            "Bearer realm=\"nvbes-account\", error=\"insufficient_scope\", scope=\"{scope}\""
        ));
        error
    }

    pub fn not_found(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, code, message)
    }

    pub fn conflict(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(StatusCode::CONFLICT, code, message)
    }

    pub fn method_not_allowed(message: impl Into<String>) -> Self {
        Self::new(
            StatusCode::METHOD_NOT_ALLOWED,
            "method_not_allowed",
            message,
        )
    }

    pub fn service_unavailable(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(StatusCode::SERVICE_UNAVAILABLE, code, message)
    }

    pub fn internal(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, code, message)
    }

    pub fn invalid_json(error: axum::extract::rejection::JsonRejection) -> Self {
        Self::bad_request("invalid_json", error.body_text())
    }

    pub fn invalid_query(error: axum::extract::rejection::QueryRejection) -> Self {
        Self::bad_request("invalid_query", error.body_text())
    }

    fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            code,
            message: message.into(),
            request_id: Uuid::new_v4(),
            authenticate: None,
        }
    }
}

impl From<sqlx::Error> for AppError {
    fn from(error: sqlx::Error) -> Self {
        tracing::error!(%error, "Account database operation failed");
        Self::internal(
            "database_error",
            "The Account service could not complete the database operation.",
        )
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let request_id = self.request_id;
        let authenticate = self.authenticate.clone();
        let body = ErrorEnvelope {
            error: ErrorBody {
                code: self.code,
                message: self.message,
                request_id,
            },
        };
        let mut response = (self.status, Json(body)).into_response();
        response.headers_mut().insert(
            "x-request-id",
            HeaderValue::from_str(&request_id.to_string())
                .expect("a UUID is always a valid header value"),
        );
        if let Some(value) = authenticate.and_then(|value| HeaderValue::from_str(&value).ok()) {
            response
                .headers_mut()
                .insert(header::WWW_AUTHENTICATE, value);
        }
        response
    }
}

#[cfg(test)]
mod tests {
    use axum::{body::to_bytes, http::StatusCode, response::IntoResponse};

    use super::AppError;

    #[tokio::test]
    async fn error_responses_have_the_typed_envelope() {
        let response = AppError::insufficient_scope("account:profile:read").into_response();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert!(response.headers().contains_key("x-request-id"));
        assert!(
            response
                .headers()
                .get("www-authenticate")
                .expect("challenge header")
                .to_str()
                .expect("header text")
                .contains("account:profile:read")
        );

        let body = to_bytes(response.into_body(), 8_192)
            .await
            .expect("response body");
        let json: serde_json::Value = serde_json::from_slice(&body).expect("JSON envelope");
        assert_eq!(json["error"]["code"], "insufficient_scope");
        assert!(json["error"]["request_id"].is_string());
    }
}
