use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, PartialEq, Eq)]
pub enum FailureResolution {
    Failed,
    Expired,
    RetryAt(DateTime<Utc>),
}

pub fn failure_resolution(
    outcome: &str,
    attempt_count: i32,
    maximum_attempts: i32,
    retry_at: DateTime<Utc>,
    deliver_before: DateTime<Utc>,
) -> FailureResolution {
    if outcome == "permanent_failure" || attempt_count >= maximum_attempts {
        FailureResolution::Failed
    } else if retry_at >= deliver_before {
        FailureResolution::Expired
    } else {
        FailureResolution::RetryAt(retry_at)
    }
}

pub async fn finish_attempt(
    pool: &PgPool,
    lease_token: Uuid,
    outcome: &'static str,
    provider_message_id: Option<&str>,
    failure_code: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE email_delivery_attempts
        SET outcome = $2::email_attempt_outcome, provider_message_id = $3,
            failure_code = $4, finished_at = clock_timestamp()
        WHERE lease_token = $1
        "#,
    )
    .bind(lease_token)
    .bind(outcome)
    .bind(provider_message_id)
    .bind(failure_code)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn finish_attempt_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    lease_token: Uuid,
    outcome: &'static str,
    provider_message_id: Option<&str>,
    failure_code: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE email_delivery_attempts
        SET outcome = $2::email_attempt_outcome, provider_message_id = $3,
            failure_code = $4, finished_at = clock_timestamp()
        WHERE lease_token = $1
        "#,
    )
    .bind(lease_token)
    .bind(outcome)
    .bind(provider_message_id)
    .bind(failure_code)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};

    use super::{FailureResolution, failure_resolution};

    #[test]
    fn retry_is_refused_at_the_delivery_deadline() {
        let now = Utc::now();
        let deadline = now + Duration::minutes(15);
        assert_eq!(
            failure_resolution("transient_failure", 1, 4, deadline, deadline),
            FailureResolution::Expired
        );
    }

    #[test]
    fn exhausted_attempts_fail_even_before_the_deadline() {
        let now = Utc::now();
        assert_eq!(
            failure_resolution(
                "transient_failure",
                4,
                4,
                now + Duration::minutes(1),
                now + Duration::minutes(15),
            ),
            FailureResolution::Failed
        );
    }
}
