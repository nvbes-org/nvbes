use std::time::Duration;

const BOT_DECISIONS_METRIC: &str = "identity_bot_guard_decisions_total";
const BOT_TARPIT_BASE_MS: u64 = 750;
const BOT_TARPIT_MAX_MS: u64 = 2_500;

pub fn record_decision(decision: &'static str, reason: &'static str) {
    metrics::counter!(
        BOT_DECISIONS_METRIC,
        &[("decision", decision), ("reason", reason)]
    )
    .increment(1);
}

pub fn tarpit_delay(score: f64) -> Duration {
    if score < 0.80 {
        return Duration::ZERO;
    }

    let normalized = score.clamp(0.80, 1.0);
    let factor = 1.0 + ((normalized - 0.80) / 0.20);
    let delay_ms = ((BOT_TARPIT_BASE_MS as f64) * factor).round() as u64;
    Duration::from_millis(delay_ms.min(BOT_TARPIT_MAX_MS))
}

pub async fn apply_tarpit(score: f64) {
    let delay = tarpit_delay(score);
    if !delay.is_zero() {
        tokio::time::sleep(delay).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tarpit_delay_is_zero_below_block_threshold() {
        assert_eq!(tarpit_delay(0.79), Duration::ZERO);
    }

    #[test]
    fn tarpit_delay_is_bounded_for_blocked_scores() {
        assert_eq!(tarpit_delay(0.80), Duration::from_millis(750));
        assert_eq!(tarpit_delay(1.00), Duration::from_millis(1_500));
        assert!(tarpit_delay(99.0) <= Duration::from_millis(BOT_TARPIT_MAX_MS));
    }
}
