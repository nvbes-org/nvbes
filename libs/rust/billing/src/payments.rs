use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaymentStatus {
    Pending,
    RequiresAction,
    Authorized,
    Captured,
    Failed,
    Refunded,
    Disputed,
    Canceled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentRecord {
    pub provider: String,
    pub status: PaymentStatus,
    pub amount_minor: i64,
    pub currency: String,
}

pub fn can_fallback_to_another_provider(status: PaymentStatus) -> bool {
    matches!(
        status,
        PaymentStatus::Pending | PaymentStatus::Failed | PaymentStatus::Canceled
    )
}

#[cfg(test)]
#[path = "payments.tests.rs"]
mod tests;
