nvbes_core::impl_app_error!();

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => Self::not_found("not_found", "Resource not found"),
            _ => Self::internal("database_error", format!("Database error: {}", err)),
        }
    }
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        Self::internal("internal_error", format!("Internal error: {}", err))
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        Self::internal("json_error", format!("JSON error: {}", err))
    }
}

impl From<nvbes_email::EmailError> for AppError {
    fn from(err: nvbes_email::EmailError) -> Self {
        Self::internal("email_error", format!("Email delivery error: {}", err))
    }
}
