#![allow(unused_imports)]
use sqlx::PgPool;
use uuid::Uuid;

use crate::{database, privacy_jobs, profile};

#[cfg(feature = "database-tests")]
#[sqlx::test(migrations = "./migrations")]
async fn connect_lazy_and_migrate_succeed_on_test_database(pool: PgPool) {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL");
    let lazy = database::connect_lazy(&url, 1).expect("lazy pool");
    assert!(lazy.acquire().await.is_ok());
    database::migrate(&pool).await.expect("migrations");
}

#[cfg(feature = "database-tests")]
#[sqlx::test(migrations = "./migrations")]
async fn account_lifecycle_is_isolated_audited_and_privacy_safe(pool: PgPool) {
    let owner = Uuid::new_v4();
    let member = Uuid::new_v4();

    profile::ensure_profile(&pool, owner)
        .await
        .expect("owner profile is provisioned");
    profile::ensure_profile(&pool, member)
        .await
        .expect("member profile is provisioned");

    sqlx::query(
        "INSERT INTO account_consents(id, principal_id, consent_type, document_version) VALUES($1,$2,'terms','v1')",
    )
    .bind(Uuid::new_v4())
    .bind(owner)
    .execute(&pool)
    .await
    .expect("consent is recorded");

    let export_id = Uuid::new_v4();
    sqlx::query("INSERT INTO account_exports(id,principal_id,status) VALUES($1,$2,'pending')")
        .bind(export_id)
        .bind(owner)
        .execute(&pool)
        .await
        .expect("export request inserts");
    let exports = privacy_jobs::process_pending(&pool)
        .await
        .expect("export job completes");
    assert_eq!(exports.exports_completed, 1);
    let export_has_consents: bool = sqlx::query_scalar(
        "SELECT status='completed' AND document IS NOT NULL AND document ? 'consents' FROM account_exports WHERE id=$1",
    )
    .bind(export_id)
    .fetch_one(&pool)
    .await
    .expect("export row is readable");
    assert!(export_has_consents);

    let member_profile_untouched: i64 =
        sqlx::query_scalar("SELECT count(*) FROM account_exports WHERE principal_id=$1")
            .bind(member)
            .fetch_one(&pool)
            .await
            .expect("member exports are countable");
    assert_eq!(member_profile_untouched, 0);

    let closure_id = Uuid::new_v4();
    sqlx::query("INSERT INTO account_closures(id,principal_id,status,requested_at,execute_after) VALUES($1,$2,'pending',clock_timestamp()-interval '8 days',clock_timestamp()-interval '1 day')")
        .bind(closure_id)
        .bind(owner)
        .execute(&pool)
        .await
        .expect("due closure inserts");
    let jobs = privacy_jobs::process_pending(&pool)
        .await
        .expect("privacy jobs complete");
    assert_eq!(jobs.closures_completed, 1);

    let lifecycle: (String, bool) = sqlx::query_as(
        "SELECT lifecycle_status, closed_at IS NOT NULL FROM account_profiles WHERE principal_id=$1",
    )
    .bind(owner)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(lifecycle, ("closed".into(), true));
    let preferences: i64 =
        sqlx::query_scalar("SELECT count(*) FROM account_preferences WHERE principal_id=$1")
            .bind(owner)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(preferences, 0);
    let retained_document: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM account_exports WHERE principal_id=$1 AND document IS NOT NULL",
    )
    .bind(owner)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(retained_document, 0);

    let event_id: Uuid = sqlx::query_scalar(
        "SELECT id FROM account_audit_events WHERE principal_id=$1 ORDER BY occurred_at LIMIT 1",
    )
    .bind(owner)
    .fetch_one(&pool)
    .await
    .unwrap();
    let mutation = sqlx::query("DELETE FROM account_audit_events WHERE id=$1")
        .bind(event_id)
        .execute(&pool)
        .await;
    assert!(
        mutation.is_err(),
        "append-only audit events reject deletion"
    );
}

#[cfg(feature = "database-tests")]
#[sqlx::test(migrations = "./migrations")]
async fn profile_preferences_and_export_jobs(pool: PgPool) {
    let owner = Uuid::new_v4();

    let created = profile::ensure_profile(&pool, owner).await.unwrap();
    assert_eq!(created.principal_id, owner);
    let again = profile::ensure_profile(&pool, owner).await.unwrap();
    assert_eq!(again.principal_id, owner);

    sqlx::query(
        "UPDATE account_preferences SET theme='dark', language='en', email_notifications=false WHERE principal_id=$1",
    )
    .bind(owner)
    .execute(&pool)
    .await
    .unwrap();
    let theme: String =
        sqlx::query_scalar("SELECT theme FROM account_preferences WHERE principal_id=$1")
            .bind(owner)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(theme, "dark");

    let export_id = Uuid::new_v4();
    sqlx::query("INSERT INTO account_exports(id,principal_id,status) VALUES($1,$2,'pending')")
        .bind(export_id)
        .bind(owner)
        .execute(&pool)
        .await
        .unwrap();
    let jobs = privacy_jobs::process_pending(&pool).await.unwrap();
    assert_eq!(jobs.exports_completed, 1);
    let status: String = sqlx::query_scalar("SELECT status FROM account_exports WHERE id=$1")
        .bind(export_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, "completed");
}
