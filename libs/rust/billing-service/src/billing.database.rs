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
#[path = "billing.database.test_support.rs"]
#[allow(
    dead_code,
    reason = "consumed by metrics/health/customer/webhook tests (some feature-gated)"
)]
pub mod test_support;

#[cfg(all(test, feature = "database-tests"))]
#[path = "billing.http.test_support.rs"]
pub mod http_test_support;

#[cfg(all(test, feature = "database-tests"))]
#[path = "billing.database.tests.rs"]
mod tests;
