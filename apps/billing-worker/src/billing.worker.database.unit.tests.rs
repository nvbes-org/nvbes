use super::{connect, connect_lazy, migrate};

#[tokio::test]
async fn connect_lazy_accepts_postgres_url_without_connecting() {
    let pool = connect_lazy("postgres://localhost/unused", 2).expect("lazy pool");
    pool.close().await;
}

#[tokio::test]
async fn connect_and_migrate_when_postgres_is_available() {
    if !postgres_reachable() {
        return;
    }
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://postgres:postgres@127.0.0.1:15432/nvbes_coverage_test".into()
    });
    let pool = connect(&url, 2).await.expect("connect");
    migrate(&pool).await.expect("migrate");
    pool.close().await;
}

#[cfg(feature = "database-tests")]
#[sqlx::test(migrations = false)]
async fn migrate_creates_the_billing_schema(pool: sqlx::PgPool) {
    let exists_before: bool = sqlx::query_scalar(
        "SELECT EXISTS (
            SELECT 1 FROM information_schema.tables
            WHERE table_schema = 'public' AND table_name = 'billing_outbox'
        )",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(
        !exists_before,
        "fixture pool must start without billing schema"
    );

    migrate(&pool).await.expect("migrate applies schema");

    let exists_after: bool = sqlx::query_scalar(
        "SELECT EXISTS (
            SELECT 1 FROM information_schema.tables
            WHERE table_schema = 'public' AND table_name = 'billing_outbox'
        )",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(exists_after);
}

fn postgres_reachable() -> bool {
    crate::test_support::postgres_reachable()
}
