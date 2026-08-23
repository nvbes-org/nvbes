use chrono::{Duration, Utc};
use nvbes_product_account::closure_event::{
    ACCOUNT_CLOSURE_REQUESTED_V1, AccountClosureRequestedV1,
};
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

use super::{apply, validate_command};

fn command(principal_id: Uuid) -> AccountClosureRequestedV1 {
    AccountClosureRequestedV1::new(Uuid::new_v4(), Uuid::new_v4(), principal_id, Utc::now())
}

#[test]
fn command_rejects_unknown_versions_and_future_dates() {
    let mut command = command(Uuid::new_v4());
    command.event_type = "account.closure.requested.v2".to_string();
    assert!(validate_command(&command).is_err());
    command.event_type = ACCOUNT_CLOSURE_REQUESTED_V1.to_string();
    command.requested_at = Utc::now() + Duration::minutes(6);
    assert!(validate_command(&command).is_err());
}

#[tokio::test]
#[ignore = "requires NVBES_BILLING_TEST_DATABASE_URL pointing to disposable PostgreSQL"]
async fn closure_purges_billing_user_and_replays_safely() {
    let database_url = std::env::var("NVBES_BILLING_TEST_DATABASE_URL")
        .expect("NVBES_BILLING_TEST_DATABASE_URL is required");
    let db = PgPoolOptions::new()
        .max_connections(2)
        .connect(&database_url)
        .await
        .expect("disposable Billing database");
    sqlx::migrate!("./migrations")
        .run(&db)
        .await
        .expect("Billing migrations");

    let principal_id = Uuid::new_v4();
    sqlx::query("INSERT INTO principals (id, display_name) VALUES ($1, 'Closure test')")
        .bind(principal_id)
        .execute(&db)
        .await
        .expect("Billing principal");
    sqlx::query("INSERT INTO users (principal_id, email, name) VALUES ($1, $2, 'Closure test')")
        .bind(principal_id)
        .bind(format!("billing-closure-{principal_id}@example.test"))
        .execute(&db)
        .await
        .expect("Billing user");
    let command = command(principal_id);

    assert!(apply(&db, &command).await.expect("first closure"));
    assert!(!apply(&db, &command).await.expect("closure replay"));
    let user_exists: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM users WHERE principal_id = $1)")
            .bind(principal_id)
            .fetch_one(&db)
            .await
            .expect("Billing user state");
    assert!(!user_exists);
    let principal_status: String =
        sqlx::query_scalar("SELECT status FROM principals WHERE id = $1")
            .bind(principal_id)
            .fetch_one(&db)
            .await
            .expect("Billing principal tombstone");
    assert_eq!(principal_status, "deleted");

    let collision = AccountClosureRequestedV1 {
        saga_id: Uuid::new_v4(),
        ..command
    };
    assert!(apply(&db, &collision).await.is_err());
}
