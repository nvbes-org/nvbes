use super::error::OAuthError;
use crate::rate_limits::{Category, LimitError, RateLimiter};
use sqlx::PgPool;

pub(super) async fn enforce(
    db: &PgPool,
    limiter: &RateLimiter,
    category: Category,
    subject: &str,
) -> Result<(), OAuthError> {
    match tokio::time::timeout(
        std::time::Duration::from_secs(2),
        limiter.check(db, category, subject),
    )
    .await
    {
        Ok(Ok(())) => Ok(()),
        Ok(Err(LimitError::Exceeded)) => Err(OAuthError::RateLimited(category.retry_after())),
        _ => Err(OAuthError::Unavailable),
    }
}
