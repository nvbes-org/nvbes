const HOSTED_AUTHORIZATION_STATE_KEY_PREFIX: &str = "nvbes:identity:oauth:hosted-login";
const HOSTED_AUTHORIZATION_STATE_TTL_SECONDS: u64 = 300;

pub fn hosted_authorization_state_key(state_id: &str) -> String {
    format!("{HOSTED_AUTHORIZATION_STATE_KEY_PREFIX}:{state_id}")
}

pub fn hosted_authorization_state_ttl_seconds() -> u64 {
    HOSTED_AUTHORIZATION_STATE_TTL_SECONDS
}
