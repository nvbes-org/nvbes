nvbes_core::impl_app_error!();

impl From<sqlx::Error> for AppError {
    fn from(error: sqlx::Error) -> Self {
        Self::internal("database_error", format!("Database error: {error}"))
    }
}
