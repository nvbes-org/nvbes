use std::{net::IpAddr, str::FromStr};

use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use uuid::Uuid;

use super::deterministic_message_id;

#[tokio::test]
async fn migration_keeps_the_previous_insert_contract_compatible() {
    let database_url = std::env::var("NVBES_DATABASE_URL").expect("NVBES_DATABASE_URL is required");
    validate_test_database_url(&database_url).expect("database must be an isolated loopback test");
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await
        .expect("test database connection");
    let mut transaction = pool.begin().await.expect("test transaction");
    let job_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO email_messages (
            job_id,
            business_type,
            recipient_email,
            recipient_hash,
            provider_email_id,
            status
        )
        VALUES ($1, 'legacy_worker', 'recipient@example.test', $2, 'legacy-provider-id', 'sent')
        "#,
    )
    .bind(job_id)
    .bind("0".repeat(64))
    .execute(&mut *transaction)
    .await
    .expect("the pre-migration worker insert shape must remain valid");

    let (message_id, sent_at) =
        sqlx::query_as::<_, (String, Option<chrono::DateTime<chrono::Utc>>)>(
            "SELECT message_id, sent_at FROM email_messages WHERE job_id = $1",
        )
        .bind(job_id)
        .fetch_one(&mut *transaction)
        .await
        .expect("legacy delivery should be queryable");

    assert_eq!(message_id, deterministic_message_id(job_id));
    assert!(sent_at.is_some());
    transaction
        .rollback()
        .await
        .expect("test transaction rollback");
}

fn validate_test_database_url(database_url: &str) -> Result<(), String> {
    if !database_url.starts_with("postgres://") && !database_url.starts_with("postgresql://") {
        return Err("database URL must use PostgreSQL".to_string());
    }
    let options = PgConnectOptions::from_str(database_url).map_err(|error| error.to_string())?;

    let host = options.get_host();
    let is_loopback = host == "localhost"
        || host
            .parse::<IpAddr>()
            .is_ok_and(|address| address.is_loopback());
    if !is_loopback {
        return Err("database URL must target loopback".to_string());
    }

    let database_name = options
        .get_database()
        .unwrap_or_default()
        .to_ascii_lowercase();
    if database_name.is_empty()
        || !database_name.contains("test")
        || ["postgres", "production", "staging", "live"].contains(&database_name.as_str())
    {
        return Err("database URL must name an isolated test database".to_string());
    }
    Ok(())
}

#[test]
fn migration_contract_rejects_non_test_or_remote_databases() {
    assert!(
        validate_test_database_url(
            "postgres://postgres:postgres@127.0.0.1:5432/nvbes_account_test"
        )
        .is_ok()
    );
    assert!(validate_test_database_url("postgres://db.internal/nvbes_account_test").is_err());
    assert!(validate_test_database_url("postgres://127.0.0.1/nvbes").is_err());
}
