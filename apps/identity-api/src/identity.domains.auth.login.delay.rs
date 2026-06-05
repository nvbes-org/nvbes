use sqlx::PgPool;
use uuid::Uuid;

use crate::http::error::AppError;

const FAILURE_THRESHOLD: u32 = 3;
const BASE_DELAY_MS: u64 = 1000;
const MAX_DELAY_MS: u64 = 60_000;

pub fn compute_progressive_delay(failures: u32) -> std::time::Duration {
    if failures <= FAILURE_THRESHOLD {
        return std::time::Duration::ZERO;
    }
    let excess = failures.saturating_sub(FAILURE_THRESHOLD);
    let multiplier = 2u64.saturating_pow(excess.saturating_sub(1));
    let delay_ms = BASE_DELAY_MS.saturating_mul(multiplier).min(MAX_DELAY_MS);
    std::time::Duration::from_millis(delay_ms)
}

pub async fn apply_login_delay(db: &PgPool, principal_id: Uuid) -> Result<(), AppError> {
    let failures: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM risk_events
        WHERE principal_id = $1
          AND event_type = 'login_failed'
          AND created_at > NOW() - INTERVAL '15 minutes'
        "#,
    )
    .bind(principal_id)
    .fetch_one(db)
    .await?;

    let delay = compute_progressive_delay(failures as u32);
    if !delay.is_zero() {
        tokio::time::sleep(delay).await;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_delay_below_threshold() {
        assert_eq!(compute_progressive_delay(0), std::time::Duration::ZERO);
        assert_eq!(compute_progressive_delay(1), std::time::Duration::ZERO);
        assert_eq!(compute_progressive_delay(2), std::time::Duration::ZERO);
        assert_eq!(compute_progressive_delay(3), std::time::Duration::ZERO);
    }

    #[test]
    fn progressive_delay_grows_exponentially() {
        assert_eq!(
            compute_progressive_delay(4),
            std::time::Duration::from_secs(1)
        );
        assert_eq!(
            compute_progressive_delay(5),
            std::time::Duration::from_secs(2)
        );
        assert_eq!(
            compute_progressive_delay(6),
            std::time::Duration::from_secs(4)
        );
        assert_eq!(
            compute_progressive_delay(7),
            std::time::Duration::from_secs(8)
        );
        assert_eq!(
            compute_progressive_delay(8),
            std::time::Duration::from_secs(16)
        );
        assert_eq!(
            compute_progressive_delay(9),
            std::time::Duration::from_secs(32)
        );
    }

    #[test]
    fn delay_capped_at_max() {
        assert_eq!(
            compute_progressive_delay(100),
            std::time::Duration::from_millis(MAX_DELAY_MS)
        );
    }
}
