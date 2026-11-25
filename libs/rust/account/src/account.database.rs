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

pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

pub async fn migrate(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    MIGRATOR.run(pool).await
}

#[cfg(all(test, feature = "database-tests"))]
#[path = "account.database.tests.rs"]
mod tests;
