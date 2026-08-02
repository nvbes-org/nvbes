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
#[ignore = "requires NVBES_CLOUD_TEST_DATABASE_URL pointing to disposable PostgreSQL"]
async fn closure_tombstones_cloud_subject_and_replays_safely() {
    let database_url = std::env::var("NVBES_CLOUD_TEST_DATABASE_URL")
        .expect("NVBES_CLOUD_TEST_DATABASE_URL is required");
    let db = PgPoolOptions::new()
        .max_connections(2)
        .connect(&database_url)
        .await
        .expect("disposable Cloud database");
    sqlx::migrate!("./migrations")
        .run(&db)
        .await
        .expect("Cloud migrations");

    let principal_id = Uuid::new_v4();
    let local_user_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO users (email, display_name, status, identity_subject)
        VALUES ($1, 'Closure test', 'active', $2)
        RETURNING id
        "#,
    )
    .bind(format!("cloud-closure-{principal_id}@example.test"))
    .bind(principal_id.to_string())
    .fetch_one(&db)
    .await
    .expect("Cloud user");
    let command = command(principal_id);

    assert!(apply(&db, &command).await.expect("first closure"));
    assert!(!apply(&db, &command).await.expect("closure replay"));
    let user: (String, String) =
        sqlx::query_as("SELECT email, status::text FROM users WHERE id = $1")
            .bind(local_user_id)
            .fetch_one(&db)
            .await
            .expect("Cloud tombstone");
    assert_eq!(user.0, format!("closed-{local_user_id}@deleted.invalid"));
    assert_eq!(user.1, "deleted");

    let collision = AccountClosureRequestedV1 {
        saga_id: Uuid::new_v4(),
        ..command
    };
    assert!(apply(&db, &collision).await.is_err());
}

#[tokio::test]
#[ignore = "requires NVBES_CLOUD_TEST_DATABASE_URL pointing to disposable PostgreSQL"]
async fn closure_is_rejected_before_inbox_commit_for_workspace_owners() {
    let database_url = std::env::var("NVBES_CLOUD_TEST_DATABASE_URL")
        .expect("NVBES_CLOUD_TEST_DATABASE_URL is required");
    let db = PgPoolOptions::new()
        .max_connections(2)
        .connect(&database_url)
        .await
        .expect("disposable Cloud database");
    sqlx::migrate!("./migrations")
        .run(&db)
        .await
        .expect("Cloud migrations");

    let principal_id = Uuid::new_v4();
    let local_user_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO users (email, display_name, status, identity_subject)
        VALUES ($1, 'Workspace owner', 'active', $2)
        RETURNING id
        "#,
    )
    .bind(format!("cloud-owner-{principal_id}@example.test"))
    .bind(principal_id.to_string())
    .fetch_one(&db)
    .await
    .expect("Cloud owner");
    let plan_id: Uuid = sqlx::query_scalar("SELECT id FROM plans ORDER BY code LIMIT 1")
        .fetch_one(&db)
        .await
        .expect("Cloud plan");
    sqlx::query(
        r#"
        INSERT INTO workspaces
          (workspace_type, name, owner_user_id, owner_principal_id, plan_id)
        VALUES ('personal', 'Owned workspace', $1, $2, $3)
        "#,
    )
    .bind(local_user_id)
    .bind(principal_id)
    .bind(plan_id)
    .execute(&db)
    .await
    .expect("owned workspace");
    let command = command(principal_id);

    assert!(apply(&db, &command).await.is_err());
    let inbox_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM cloud_account_closure_inbox WHERE event_id = $1)",
    )
    .bind(command.event_id)
    .fetch_one(&db)
    .await
    .expect("Cloud inbox state");
    assert!(!inbox_exists);
}
