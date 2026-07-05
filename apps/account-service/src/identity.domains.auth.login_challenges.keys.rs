use chrono::{DateTime, Utc};
use uuid::Uuid;

pub(super) fn challenge_key(challenge_id: &str) -> String {
    format!("nvbes:identity:login-challenge:{challenge_id}")
}

pub(super) fn auth_state_index_id(auth_state_id: Uuid) -> String {
    format!("nvbes:identity:login-challenges:auth-state:{auth_state_id}")
}

pub(super) fn active_challenge_key(auth_state_id: Uuid, purpose: &str) -> String {
    format!("nvbes:identity:login-challenges:active:{auth_state_id}:{purpose}")
}

pub(super) fn factor_allowed(allowed_factor_types: &[String], factor_type: &str) -> bool {
    allowed_factor_types.is_empty()
        || allowed_factor_types
            .iter()
            .any(|value| value == factor_type)
}

pub(super) fn challenge_ttl_seconds(expires_at: DateTime<Utc>) -> u64 {
    (expires_at - Utc::now()).num_seconds().max(1) as u64
}
