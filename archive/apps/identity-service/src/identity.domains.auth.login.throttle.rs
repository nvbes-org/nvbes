use std::time::Duration;

use nvbes_core::limiter::RateLimitRule;
use uuid::Uuid;

pub const LOGIN_THROTTLE_ACTION: &str = "auth_login";
pub const LOGIN_THROTTLE_WINDOW: Duration = Duration::from_secs(300);
const LOGIN_ACCOUNT_MAX_HITS: usize = 30;
const LOGIN_IP_ACCOUNT_MAX_HITS: usize = 15;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginThrottleKeys {
    pub ip: String,
    pub account: String,
    pub ip_account_pair: String,
    pub tenant: Option<String>,
}

impl LoginThrottleKeys {
    pub fn from_parts(ip: Option<&str>, normalized_email: &str, tenant_id: Option<Uuid>) -> Self {
        let ip_val = ip.unwrap_or("unknown");
        Self {
            ip: format!("ip:{ip_val}"),
            account: format!("account:{normalized_email}"),
            ip_account_pair: format!("ip_account:{ip_val}:{normalized_email}"),
            tenant: tenant_id.map(|id| format!("tenant:{id}")),
        }
    }

    pub fn pre_lookup_rules(&self) -> [RateLimitRule<'_>; 3] {
        [
            RateLimitRule {
                key: &self.ip,
                max_hits: 60,
                window: LOGIN_THROTTLE_WINDOW,
            },
            RateLimitRule {
                key: &self.account,
                max_hits: LOGIN_ACCOUNT_MAX_HITS,
                window: LOGIN_THROTTLE_WINDOW,
            },
            RateLimitRule {
                key: &self.ip_account_pair,
                max_hits: LOGIN_IP_ACCOUNT_MAX_HITS,
                window: LOGIN_THROTTLE_WINDOW,
            },
        ]
    }

    pub fn tenant_rule(&self) -> Option<RateLimitRule<'_>> {
        self.tenant.as_ref().map(|key| RateLimitRule {
            key,
            max_hits: 120,
            window: LOGIN_THROTTLE_WINDOW,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_stable_ip_account_and_tenant_keys() {
        let tenant_id = Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap();

        let keys = LoginThrottleKeys::from_parts(
            Some("203.0.113.10"),
            "USER@example.com",
            Some(tenant_id),
        );

        assert_eq!(keys.ip, "ip:203.0.113.10");
        assert_eq!(keys.account, "account:USER@example.com");
        assert_eq!(
            keys.ip_account_pair,
            "ip_account:203.0.113.10:USER@example.com"
        );
        assert_eq!(
            keys.tenant,
            Some("tenant:11111111-1111-1111-1111-111111111111".to_string())
        );

        let rules = keys.pre_lookup_rules();
        assert_eq!(rules.len(), 3);
        assert_eq!(rules[0].key, "ip:203.0.113.10");
        assert_eq!(rules[1].key, "account:USER@example.com");
        assert_eq!(rules[1].max_hits, 30);
        assert_eq!(rules[2].key, "ip_account:203.0.113.10:USER@example.com");
        assert_eq!(rules[2].max_hits, 15);
    }

    #[test]
    fn uses_unknown_ip_without_dropping_account_limit() {
        let keys = LoginThrottleKeys::from_parts(None, "user@example.com", None);

        assert_eq!(keys.ip, "ip:unknown");
        assert_eq!(keys.account, "account:user@example.com");
        assert_eq!(keys.ip_account_pair, "ip_account:unknown:user@example.com");
        assert_eq!(keys.tenant, None);
    }
}
