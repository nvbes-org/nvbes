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

impl From<nvbes_billing::usage::BillingUsageIngestError> for AppError {
    fn from(err: nvbes_billing::usage::BillingUsageIngestError) -> Self {
        match err {
            nvbes_billing::usage::BillingUsageIngestError::Validation { code, message } => {
                Self::bad_request(code, message)
            }
            nvbes_billing::usage::BillingUsageIngestError::Database(err) => err.into(),
            nvbes_billing::usage::BillingUsageIngestError::Serialization(err) => err.into(),
        }
    }
}

impl From<nvbes_billing::reconciliation_db::BillingReconciliationError> for AppError {
    fn from(err: nvbes_billing::reconciliation_db::BillingReconciliationError) -> Self {
        match err {
            nvbes_billing::reconciliation_db::BillingReconciliationError::Database(err) => {
                err.into()
            }
        }
    }
}
