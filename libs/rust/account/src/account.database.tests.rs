#![allow(unused_imports)]
use sqlx::PgPool;
use uuid::Uuid;

use crate::{database, privacy_jobs, profile, synthetic, teams};

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

    let result = synthetic::run(&pool, owner, member)
        .await
        .expect("synthetic Account lifecycle succeeds");

    assert_eq!(result.member_role, "member");
    assert_eq!(result.team_members, 2);
    assert!(result.member_left);
    assert!(result.consent_granted);
    assert!(result.export_completed);
    assert!(result.closure_cancelled);
    assert!(result.audit_events >= 5);
    assert!(result.outbox_events >= 3);
    assert!(result.outbox_published);

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
async fn profile_preferences_teams_and_export_jobs(pool: PgPool) {
    let owner = Uuid::new_v4();
    let member = Uuid::new_v4();

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

    let (team_id, role) = teams::create_and_join_for_synthetic(&pool, owner, member)
        .await
        .unwrap();
    assert_eq!(role, "member");
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM account_team_memberships WHERE team_id=$1",
        )
        .bind(team_id)
        .fetch_one(&pool)
        .await
        .unwrap(),
        2
    );

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

#[cfg(feature = "database-tests")]
#[sqlx::test(migrations = "./migrations")]
async fn join_team_rejects_malformed_codes_and_is_idempotent(pool: PgPool) {
    use sha2::{Digest, Sha256};

    let owner = Uuid::new_v4();
    let member = Uuid::new_v4();
    profile::ensure_profile(&pool, owner).await.unwrap();
    profile::ensure_profile(&pool, member).await.unwrap();

    let join_code = format!("team_{}", Uuid::new_v4().simple());
    assert_eq!(join_code.len(), 37);
    let team_id = Uuid::new_v4();
    let hash = Sha256::digest(join_code.as_bytes());
    sqlx::query(
        "INSERT INTO account_teams(id, owner_principal_id, name, join_code_hash) VALUES($1,$2,'Join suite',$3)",
    )
    .bind(team_id)
    .bind(owner)
    .bind(hash.as_slice())
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO account_team_memberships(team_id,principal_id,role) VALUES($1,$2,'owner')",
    )
    .bind(team_id)
    .bind(owner)
    .execute(&pool)
    .await
    .unwrap();

    let first = teams::join_team(&pool, member, &join_code, Uuid::new_v4())
        .await
        .unwrap();
    assert_eq!(first.id, team_id);
    assert_eq!(first.role, "member");

    let second = teams::join_team(&pool, member, &join_code, Uuid::new_v4())
        .await
        .unwrap();
    assert_eq!(second.id, team_id);
    assert_eq!(second.role, "member");

    assert!(matches!(
        teams::join_team(&pool, member, "bad", Uuid::new_v4()).await,
        Err(crate::error::AccountError::NotFound)
    ));
    assert!(matches!(
        teams::join_team(&pool, member, "team_short", Uuid::new_v4()).await,
        Err(crate::error::AccountError::NotFound)
    ));
}
