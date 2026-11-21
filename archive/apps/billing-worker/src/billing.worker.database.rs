use sqlx::{PgPool, postgres::PgPoolOptions};

pub fn connect_lazy(database_url: &str, max_connections: u32) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .min_connections(0)
        .max_connections(max_connections)
        .connect_lazy(database_url)
}

pub async fn connect(database_url: &str, max_connections: u32) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .min_connections(0)
        .max_connections(max_connections)
        .connect(database_url)
        .await
}

pub async fn migrate(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("./migrations").run(pool).await
}

#[cfg(test)]
#[path = "billing.worker.database.unit.tests.rs"]
mod unit_tests;

#[cfg(all(test, feature = "database-tests"))]
#[path = "billing.worker.database.tests.rs"]
mod tests;
