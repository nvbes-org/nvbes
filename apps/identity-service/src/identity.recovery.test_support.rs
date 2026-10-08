use crate::{auth, database};
use sqlx::{PgPool, postgres::PgPoolOptions};
use uuid::Uuid;
pub(crate) const INITIAL: &str = "Initial-recovery-database-password!";
pub(crate) async fn fixture() -> (PgPool, Uuid, String) {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL required");
    let admin = database::connect(&url, 1).await.unwrap();
    let schema = format!("identity_password_recovery_{}", Uuid::new_v4().simple());
    sqlx::query(&format!("CREATE SCHEMA {schema}"))
        .execute(&admin)
        .await
        .unwrap();
    admin.close().await;
    let db = PgPoolOptions::new()
        .max_connections(4)
        .after_connect(move |connection, _| {
            let schema = schema.clone();
            Box::pin(async move {
                sqlx::query("SELECT set_config('search_path',$1,false),set_config('application_name',$1,false)")
                    .bind(schema)
                    .execute(connection)
                    .await?;
                Ok(())
            })
        })
        .connect(&url)
        .await
        .unwrap();
    database::migrate(&db).await.unwrap();
    let email = format!("recovery-{}@example.invalid", Uuid::new_v4());
    let principal = auth::create_synthetic_identity(&db, &email, INITIAL)
        .await
        .unwrap();
    (db, principal, email)
}
