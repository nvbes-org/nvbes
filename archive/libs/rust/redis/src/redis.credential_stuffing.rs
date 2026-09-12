use redis::AsyncCommands;

use crate::connection::{RedisError, RedisPool};

const SHORT_WINDOW_SECONDS: u64 = 5 * 60;
const LONG_WINDOW_SECONDS: u64 = 24 * 60 * 60;
const KEY_PREFIX: &str = "nvbes:identity:credential-stuffing:v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialStuffingDecision {
    Allow,
    StepUp,
    Block,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CredentialStuffingStats {
    pub ip_failed_accounts_short: u64,
    pub ip_failed_accounts_long: u64,
    pub account_failed_sources_short: u64,
    pub account_failed_sources_long: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CredentialStuffingAssessment {
    pub decision: CredentialStuffingDecision,
    pub score: u8,
    pub reasons: Vec<&'static str>,
    pub stats: CredentialStuffingStats,
}

pub async fn record_failed_login(
    pool: &RedisPool,
    ip: Option<&str>,
    account: &str,
) -> Result<CredentialStuffingAssessment, RedisError> {
    let ip = normalize_ip(ip);
    let account = normalize_account(account);
    let mut conn = pool.get().await?;

    for (window, ttl) in [
        ("short", SHORT_WINDOW_SECONDS),
        ("long", LONG_WINDOW_SECONDS),
    ] {
        let ip_accounts_key = ip_accounts_key(window, &ip);
        let account_sources_key = account_sources_key(window, &account);
        let _: bool = conn.sadd(&ip_accounts_key, &account).await?;
        let _: bool = conn.sadd(&account_sources_key, &ip).await?;
        let _: bool = conn.expire(&ip_accounts_key, ttl as i64).await?;
        let _: bool = conn.expire(&account_sources_key, ttl as i64).await?;
    }

    let stats = CredentialStuffingStats {
        ip_failed_accounts_short: conn.scard(ip_accounts_key("short", &ip)).await?,
        ip_failed_accounts_long: conn.scard(ip_accounts_key("long", &ip)).await?,
        account_failed_sources_short: conn.scard(account_sources_key("short", &account)).await?,
        account_failed_sources_long: conn.scard(account_sources_key("long", &account)).await?,
    };

    Ok(assess_stats(stats))
}

pub async fn assess_current(
    pool: &RedisPool,
    ip: Option<&str>,
    account: &str,
) -> Result<CredentialStuffingAssessment, RedisError> {
    let ip = normalize_ip(ip);
    let account = normalize_account(account);
    let mut conn = pool.get().await?;
    let stats = CredentialStuffingStats {
        ip_failed_accounts_short: conn.scard(ip_accounts_key("short", &ip)).await?,
        ip_failed_accounts_long: conn.scard(ip_accounts_key("long", &ip)).await?,
        account_failed_sources_short: conn.scard(account_sources_key("short", &account)).await?,
        account_failed_sources_long: conn.scard(account_sources_key("long", &account)).await?,
    };

    Ok(assess_stats(stats))
}

pub fn assess_stats(stats: CredentialStuffingStats) -> CredentialStuffingAssessment {
    let mut score = 0u8;
    let mut reasons = Vec::new();

    add_signal(
        &mut score,
        &mut reasons,
        stats.ip_failed_accounts_short >= 8,
        45,
        "ip_failed_many_accounts_short",
    );
    add_signal(
        &mut score,
        &mut reasons,
        stats.ip_failed_accounts_long >= 30,
        35,
        "ip_failed_many_accounts_long",
    );
    add_signal(
        &mut score,
        &mut reasons,
        stats.account_failed_sources_short >= 5,
        45,
        "account_failed_many_sources_short",
    );
    add_signal(
        &mut score,
        &mut reasons,
        stats.account_failed_sources_long >= 16,
        35,
        "account_failed_many_sources_long",
    );

    let decision = if score >= 70 {
        CredentialStuffingDecision::Block
    } else if score >= 35 {
        CredentialStuffingDecision::StepUp
    } else {
        CredentialStuffingDecision::Allow
    };

    CredentialStuffingAssessment {
        decision,
        score,
        reasons,
        stats,
    }
}

fn add_signal(
    score: &mut u8,
    reasons: &mut Vec<&'static str>,
    condition: bool,
    weight: u8,
    reason: &'static str,
) {
    if condition {
        *score = score.saturating_add(weight);
        reasons.push(reason);
    }
}

fn ip_accounts_key(window: &str, ip: &str) -> String {
    format!("{KEY_PREFIX}:{window}:ip:{ip}:accounts")
}

fn account_sources_key(window: &str, account: &str) -> String {
    format!("{KEY_PREFIX}:{window}:account:{account}:sources")
}

fn normalize_ip(ip: Option<&str>) -> String {
    ip.map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("unknown")
        .chars()
        .map(safe_key_char)
        .take(96)
        .collect()
}

fn normalize_account(account: &str) -> String {
    account
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(safe_key_char)
        .take(160)
        .collect()
}

fn safe_key_char(value: char) -> char {
    if value.is_ascii_alphanumeric() || matches!(value, '.' | '-' | '_' | ':' | '@') {
        value
    } else {
        '_'
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assess_blocks_when_ip_and_account_patterns_stack() {
        let assessment = assess_stats(CredentialStuffingStats {
            ip_failed_accounts_short: 9,
            ip_failed_accounts_long: 35,
            account_failed_sources_short: 1,
            account_failed_sources_long: 1,
        });

        assert_eq!(assessment.decision, CredentialStuffingDecision::Block);
        assert!(
            assessment
                .reasons
                .contains(&"ip_failed_many_accounts_short")
        );
    }

    #[test]
    fn assess_steps_up_on_single_medium_signal() {
        let assessment = assess_stats(CredentialStuffingStats {
            ip_failed_accounts_short: 8,
            ip_failed_accounts_long: 1,
            account_failed_sources_short: 1,
            account_failed_sources_long: 1,
        });

        assert_eq!(assessment.decision, CredentialStuffingDecision::StepUp);
    }
}
