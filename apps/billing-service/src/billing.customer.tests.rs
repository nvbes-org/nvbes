use sqlx::PgPool;
use uuid::Uuid;

use super::get_or_create_customer;
use crate::database::test_support::test_config;

#[sqlx::test(migrations = "./migrations")]
async fn get_or_create_is_idempotent_for_account(pool: PgPool) {
    let config = test_config();
    let account_id = Uuid::new_v4();

    let first = get_or_create_customer(&pool, &config, account_id, "principal", Some("a@t.test"))
        .await
        .expect("create");
    let second = get_or_create_customer(&pool, &config, account_id, "principal", Some("a@t.test"))
        .await
        .expect("reuse");

    assert_eq!(first, second);
    assert!(first.starts_with("cus_test_"));

    let rows: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM billing_customers WHERE account_id = $1 AND account_type = 'principal'",
    )
    .bind(account_id)
    .fetch_one(&pool)
    .await
    .expect("count");
    assert_eq!(rows, 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn get_or_create_accepts_missing_email(pool: PgPool) {
    let config = test_config();
    let account_id = Uuid::new_v4();
    let customer_id = get_or_create_customer(&pool, &config, account_id, "team", None)
        .await
        .expect("create without email");
    assert!(customer_id.starts_with("cus_test_"));
}
