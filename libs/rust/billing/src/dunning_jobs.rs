use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DunningProcessingRun {
    pub attempts_processed: i64,
}

#[derive(Debug, Error)]
pub enum DunningProcessingError {
    #[error("invalid dunning batch size")]
    InvalidBatchSize,
    #[error("dunning processing query failed: {0}")]
    Database(#[from] sqlx::Error),
}

pub async fn process_due_dunning_attempts(
    db: &sqlx::PgPool,
    batch_size: i64,
) -> Result<DunningProcessingRun, DunningProcessingError> {
    if !(1..=500).contains(&batch_size) {
        return Err(DunningProcessingError::InvalidBatchSize);
    }

    let attempts_processed = sqlx::query_scalar::<_, i64>(
        r#"
        WITH due AS (
          SELECT attempt.id, attempt.dunning_case_id
          FROM billing_dunning_attempts attempt
          INNER JOIN billing_dunning_cases dunning_case
            ON dunning_case.id = attempt.dunning_case_id
          WHERE attempt.status = 'pending'
            AND attempt.scheduled_at <= NOW()
            AND dunning_case.status = 'open'
          ORDER BY attempt.scheduled_at ASC, attempt.created_at ASC
          LIMIT $1
          FOR UPDATE OF attempt SKIP LOCKED
        ),
        completed AS (
          UPDATE billing_dunning_attempts attempt
          SET status = 'completed',
              completed_at = NOW()
          FROM due
          WHERE attempt.id = due.id
          RETURNING due.dunning_case_id
        ),
        touched_cases AS (
          UPDATE billing_dunning_cases dunning_case
          SET updated_at = NOW()
          WHERE dunning_case.id IN (SELECT dunning_case_id FROM completed)
          RETURNING dunning_case.id
        )
        SELECT COUNT(*)::BIGINT FROM completed
        "#,
    )
    .bind(batch_size)
    .fetch_one(db)
    .await?;

    Ok(DunningProcessingRun { attempts_processed })
}

#[cfg(test)]
mod tests {
    use super::{DunningProcessingError, process_due_dunning_attempts};

    #[tokio::test]
    async fn rejects_invalid_batch_size_before_database_access() {
        let err = process_due_dunning_attempts(
            &sqlx::PgPool::connect_lazy("postgres://localhost/unused").expect("pool"),
            0,
        )
        .await
        .expect_err("batch size must be rejected");

        assert!(matches!(err, DunningProcessingError::InvalidBatchSize));
    }
}
