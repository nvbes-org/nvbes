use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccessPolicyState {
    Active,
    Warning,
    Grace,
    Degraded,
    Suspended,
    Canceled,
}

pub fn policy_after_payment_failure(previous_failures: u32) -> AccessPolicyState {
    match previous_failures {
        0 => AccessPolicyState::Warning,
        1..=2 => AccessPolicyState::Grace,
        3..=4 => AccessPolicyState::Degraded,
        _ => AccessPolicyState::Suspended,
    }
}

pub fn policy_after_payment_success() -> AccessPolicyState {
    AccessPolicyState::Active
}
