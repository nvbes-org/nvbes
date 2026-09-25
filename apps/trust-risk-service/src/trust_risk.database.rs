use sqlx::{PgPool, postgres::PgPoolOptions};

pub fn connect_lazy(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(20)
        .min_connections(0)
        .connect_lazy(database_url)
}

pub async fn migrate(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("./migrations").run(pool).await
}

pub async fn database_now(pool: &PgPool) -> Result<chrono::DateTime<chrono::Utc>, sqlx::Error> {
    sqlx::query_scalar("SELECT clock_timestamp()")
        .fetch_one(pool)
        .await
}

#[cfg(all(test, feature = "database-tests"))]
mod tests {
    use super::migrate;

    #[sqlx::test(migrations = false)]
    async fn migrate_creates_trust_risk_schema(pool: sqlx::PgPool) {
        let exists_before: bool = sqlx::query_scalar(
            "SELECT EXISTS (
                SELECT 1 FROM information_schema.tables
                WHERE table_schema = 'public' AND table_name = 'trust_risk_signals'
            )",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(!exists_before);

        migrate(&pool).await.expect("migrate");

        let exists_after: bool = sqlx::query_scalar(
            "SELECT EXISTS (
                SELECT 1 FROM information_schema.tables
                WHERE table_schema = 'public' AND table_name = 'trust_risk_signals'
            )",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(exists_after);
    }
}
