nvbes_core::impl_app_error!();

impl From<sqlx::Error> for AppError {
    fn from(error: sqlx::Error) -> Self {
        match error {
            sqlx::Error::RowNotFound => Self::not_found("not_found", "Resource not found."),
            _ => Self::internal("database_error", error.to_string()),
        }
    }
}

impl From<reqwest::Error> for AppError {
    fn from(error: reqwest::Error) -> Self {
        Self::internal("identity_request_failed", error.to_string())
    }
}

impl From<tonic::Status> for AppError {
    fn from(status: tonic::Status) -> Self {
        use tonic::Code;

        match status.code() {
            Code::InvalidArgument => Self::bad_request("validation_failed", status.message()),
            Code::NotFound => Self::not_found("not_found", status.message()),
            Code::AlreadyExists | Code::Aborted => Self::conflict("conflict", status.message()),
            Code::PermissionDenied => Self::forbidden("forbidden", status.message()),
            Code::Unauthenticated => Self::unauthorized("unauthorized", status.message()),
            _ => Self::internal("developer_operation_failed", status.message()),
        }
    }
}

impl From<nvbes_product_identity::IdentityError> for AppError {
    fn from(error: nvbes_product_identity::IdentityError) -> Self {
        use nvbes_product_identity::IdentityErrorKind::*;
        match error.kind {
            BadRequest => Self::bad_request(error.code, error.message),
            Unauthorized => Self::unauthorized(error.code, error.message),
            Forbidden => Self::forbidden(error.code, error.message),
            NotFound => Self::not_found(error.code, error.message),
            Conflict => Self::conflict(error.code, error.message),
            Internal => Self::internal(error.code, error.message),
        }
    }
}
