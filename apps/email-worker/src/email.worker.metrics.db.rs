use sqlx::PgPool;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QueueObservabilitySnapshot {
    pub depth: i64,
    pub oldest_age_seconds: f64,
}

pub async fn queue_observability_snapshot(
    pool: &PgPool,
) -> Result<QueueObservabilitySnapshot, sqlx::Error> {
    let (depth, oldest_age_seconds) = sqlx::query_as::<_, (i64, f64)>(
        r#"
        SELECT
            COUNT(*)::BIGINT,
            COALESCE(
                EXTRACT(EPOCH FROM (clock_timestamp() - MIN(accepted_at))),
                0
            )::DOUBLE PRECISION
        FROM email_messages
        WHERE state IN ('accepted', 'deferred', 'dispatching')
        "#,
    )
    .fetch_one(pool)
    .await?;

    Ok(QueueObservabilitySnapshot {
        depth,
        oldest_age_seconds: oldest_age_seconds.max(0.0),
    })
}

#[cfg(all(test, feature = "database-tests"))]
mod tests {
    use crate::{crypto::EmailCrypto, database, test_support};

    #[sqlx::test(migrations = "./migrations")]
    async fn queue_snapshot_tracks_pending_messages_without_sensitive_dimensions(
        pool: sqlx::PgPool,
    ) {
        let command = test_support::command("metrics-queue", "metrics@example.com");
        database::accept_command(
            &pool,
            &EmailCrypto::new([7; 32], [9; 32]),
            "nvbes.fr",
            &command.clone().into_proto(),
            &command,
        )
        .await
        .unwrap();

        let snapshot = super::queue_observability_snapshot(&pool).await.unwrap();
        assert_eq!(snapshot.depth, 1);
        assert!(snapshot.oldest_age_seconds >= 0.0);
    }
}
