#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityErrorKind {
    BadRequest,
    Unauthorized,
    Forbidden,
    NotFound,
    Conflict,
    Internal,
}

#[derive(Debug, Clone, thiserror::Error)]
#[error("{code}: {message}")]
pub struct IdentityError {
    pub kind: IdentityErrorKind,
    pub code: String,
    pub message: String,
}

pub type IdentityResult<T> = Result<T, IdentityError>;

impl IdentityError {
    pub fn new(
        kind: IdentityErrorKind,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            code: code.into(),
            message: message.into(),
        }
    }

    pub fn bad_request(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(IdentityErrorKind::BadRequest, code, message)
    }

    pub fn unauthorized(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(IdentityErrorKind::Unauthorized, code, message)
    }

    pub fn forbidden(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(IdentityErrorKind::Forbidden, code, message)
    }

    pub fn not_found(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(IdentityErrorKind::NotFound, code, message)
    }

    pub fn conflict(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(IdentityErrorKind::Conflict, code, message)
    }

    pub fn internal(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(IdentityErrorKind::Internal, code, message)
    }
}

impl From<sqlx::Error> for IdentityError {
    fn from(error: sqlx::Error) -> Self {
        match error {
            sqlx::Error::RowNotFound => Self::not_found("not_found", "Resource not found"),
            _ => Self::internal("database_error", error.to_string()),
        }
    }
}

impl From<serde_json::Error> for IdentityError {
    fn from(error: serde_json::Error) -> Self {
        Self::internal("json_error", error.to_string())
    }
}

impl From<nvbes_email::EmailError> for IdentityError {
    fn from(error: nvbes_email::EmailError) -> Self {
        Self::internal("email_error", error.to_string())
    }
}
