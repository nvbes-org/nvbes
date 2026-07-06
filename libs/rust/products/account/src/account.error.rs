#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountErrorKind {
    BadRequest,
    Unauthorized,
    Forbidden,
    NotFound,
    Conflict,
    Internal,
}

#[derive(Debug, Clone, thiserror::Error)]
#[error("{code}: {message}")]
pub struct AccountError {
    pub kind: AccountErrorKind,
    pub code: String,
    pub message: String,
}

pub type AccountResult<T> = Result<T, AccountError>;

impl AccountError {
    pub fn new(
        kind: AccountErrorKind,
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
        Self::new(AccountErrorKind::BadRequest, code, message)
    }

    pub fn unauthorized(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(AccountErrorKind::Unauthorized, code, message)
    }

    pub fn forbidden(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(AccountErrorKind::Forbidden, code, message)
    }

    pub fn not_found(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(AccountErrorKind::NotFound, code, message)
    }

    pub fn conflict(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(AccountErrorKind::Conflict, code, message)
    }

    pub fn internal(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(AccountErrorKind::Internal, code, message)
    }
}

impl From<sqlx::Error> for AccountError {
    fn from(error: sqlx::Error) -> Self {
        match error {
            sqlx::Error::RowNotFound => Self::not_found("not_found", "Resource not found"),
            _ => Self::internal("database_error", error.to_string()),
        }
    }
}

impl From<serde_json::Error> for AccountError {
    fn from(error: serde_json::Error) -> Self {
        Self::internal("json_error", error.to_string())
    }
}

impl From<nvbes_email::EmailError> for AccountError {
    fn from(error: nvbes_email::EmailError) -> Self {
        Self::internal("email_error", error.to_string())
    }
}
