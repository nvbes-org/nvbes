use crate::ingress_db::PersistSignalError;

#[derive(Debug, thiserror::Error)]
pub enum AssessmentPersistenceError {
    #[error("assessment key conflicts with different content")]
    Conflict,
    #[error("no valid active rule set is available")]
    NoActiveRules,
    #[error("active rule set is invalid")]
    InvalidRules,
    #[error("feature state is invalid")]
    CorruptFeatureState,
    #[error("evaluation ledger is invalid")]
    CorruptLedger,
    #[error(transparent)]
    Signal(#[from] PersistSignalError),
    #[error("assessment database operation failed")]
    Database(#[from] sqlx::Error),
}
