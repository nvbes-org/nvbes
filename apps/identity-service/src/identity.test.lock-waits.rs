use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres, Transaction};
use std::time::Duration;

/// Observe an actual PostgreSQL waiter, then let DB time cross the deadline.
/// Merely delaying a task would not prove it passed its initial authorization.
pub async fn release_after_deadline(
    db: &PgPool,
    mut blocker: Transaction<'_, Postgres>,
    deadline: DateTime<Utc>,
) {
    let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *blocker)
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let waiting: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE $1=ANY(pg_blocking_pids(pid)))",
            )
            .bind(pid)
            .fetch_one(db)
            .await
            .unwrap();
            if waiting {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("authentication never reached the held row lock");
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let expired: bool = sqlx::query_scalar("SELECT clock_timestamp()>=$1")
                .bind(deadline)
                .fetch_one(db)
                .await
                .unwrap();
            if expired {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("database deadline did not expire");
    blocker.rollback().await.unwrap();
}
