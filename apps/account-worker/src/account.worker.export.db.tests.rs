use std::time::Duration;

use chrono::Utc;
use nvbes_product_account::export_event::{AccountExportFragmentV1, AccountExportRequestedV1};
use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

use super::{build_account_fragment, claim, complete};

#[tokio::test]
#[ignore = "requires NVBES_ACCOUNT_TEST_DATABASE_URL pointing to disposable PostgreSQL"]
async fn resumes_each_participant_and_finalizes_one_document() {
    let url = std::env::var("NVBES_ACCOUNT_TEST_DATABASE_URL")
        .expect("NVBES_ACCOUNT_TEST_DATABASE_URL is required");
    let db = PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .unwrap();
    sqlx::migrate!("../account-service-next/migrations")
        .run(&db)
        .await
        .unwrap();
    let principal_id = Uuid::new_v4();
    let export_id = Uuid::new_v4();
    let event_id = Uuid::new_v4();
    let requested_at = Utc::now();
    sqlx::query("INSERT INTO account_profiles (principal_id) VALUES ($1)")
        .bind(principal_id)
        .execute(&db)
        .await
        .unwrap();
    sqlx::query("INSERT INTO account_privacy_exports (id, principal_id) VALUES ($1, $2)")
        .bind(export_id)
        .bind(principal_id)
        .execute(&db)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO account_export_participants (export_id, participant, ordinal) VALUES ($1, 'cloud', 10), ($1, 'billing', 20), ($1, 'identity', 30), ($1, 'account', 40)",
    )
    .bind(export_id)
    .execute(&db)
    .await
    .unwrap();
    let command = AccountExportRequestedV1::new(event_id, export_id, principal_id, requested_at);
    sqlx::query("INSERT INTO account_outbox_events (id, aggregate_type, aggregate_id, event_type, payload) VALUES ($1, 'account_export', $2, $3, $4)")
        .bind(event_id)
        .bind(export_id)
        .bind(&command.event_type)
        .bind(serde_json::to_value(&command).unwrap())
        .execute(&db)
        .await
        .unwrap();

    for expected in ["cloud", "billing", "identity", "account"] {
        let claimed = claim(&db, Duration::from_secs(30), 3)
            .await
            .unwrap()
            .expect("claimed export participant");
        assert_eq!(claimed.participant.as_str(), expected);
        let fragment = if expected == "account" {
            build_account_fragment(&db, principal_id).await.unwrap()
        } else {
            serde_json::to_value(AccountExportFragmentV1::new(
                expected,
                principal_id,
                json!({"records": []}),
            ))
            .unwrap()
        };
        complete(&db, &claimed, fragment).await.unwrap();
    }

    let document: serde_json::Value = sqlx::query_scalar(
        "SELECT document FROM account_privacy_exports WHERE id = $1 AND status = 'completed'",
    )
    .bind(export_id)
    .fetch_one(&db)
    .await
    .unwrap();
    assert_eq!(document["schema_version"], "nvbes-account-export.v1");
    for participant in ["cloud", "billing", "identity", "account"] {
        assert!(document["products"].get(participant).is_some());
    }
    assert_eq!(document["coverage"]["secrets_excluded"], true);
}
