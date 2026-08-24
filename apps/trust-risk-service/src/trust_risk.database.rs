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
