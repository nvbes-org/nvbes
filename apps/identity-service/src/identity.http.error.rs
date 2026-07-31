nvbes_core::impl_app_error!();

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => Self::not_found("not_found", "Resource not found"),
            _ => Self::internal("database_error", err.to_string()),
        }
    }
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        Self::internal("internal_error", err.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        Self::internal("json_error", err.to_string())
    }
}

impl From<nvbes_email::EmailError> for AppError {
    fn from(err: nvbes_email::EmailError) -> Self {
        Self::internal("email_error", err.to_string())
    }
}

impl From<nvbes_product_account::AccountError> for AppError {
    fn from(err: nvbes_product_account::AccountError) -> Self {
        let status = match err.kind {
            nvbes_product_account::error::AccountErrorKind::BadRequest => StatusCode::BAD_REQUEST,
            nvbes_product_account::error::AccountErrorKind::Unauthorized => {
                StatusCode::UNAUTHORIZED
            }
            nvbes_product_account::error::AccountErrorKind::Forbidden => StatusCode::FORBIDDEN,
            nvbes_product_account::error::AccountErrorKind::NotFound => StatusCode::NOT_FOUND,
            nvbes_product_account::error::AccountErrorKind::Conflict => StatusCode::CONFLICT,
            nvbes_product_account::error::AccountErrorKind::Internal => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        };

        Self::new(status, err.code, err.message)
    }
}

impl From<nvbes_product_identity::IdentityError> for AppError {
    fn from(err: nvbes_product_identity::IdentityError) -> Self {
        let status = match err.kind {
            nvbes_product_identity::IdentityErrorKind::BadRequest => StatusCode::BAD_REQUEST,
            nvbes_product_identity::IdentityErrorKind::Unauthorized => StatusCode::UNAUTHORIZED,
            nvbes_product_identity::IdentityErrorKind::Forbidden => StatusCode::FORBIDDEN,
            nvbes_product_identity::IdentityErrorKind::NotFound => StatusCode::NOT_FOUND,
            nvbes_product_identity::IdentityErrorKind::Conflict => StatusCode::CONFLICT,
            nvbes_product_identity::IdentityErrorKind::Internal => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        };

        Self::new(status, err.code, err.message)
    }
}
