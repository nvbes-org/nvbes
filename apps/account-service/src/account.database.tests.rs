use sqlx::PgPool;
use uuid::Uuid;

use crate::{privacy_jobs, synthetic};

#[sqlx::test(migrations = "./migrations")]
async fn account_lifecycle_is_isolated_audited_and_privacy_safe(pool: PgPool) {
    let owner = Uuid::new_v4();
    let member = Uuid::new_v4();

    let result = synthetic::run(&pool, owner, member)
        .await
        .expect("synthetic Account lifecycle succeeds");

    assert_eq!(result.member_role, "member");
    assert_eq!(result.team_members, 2);
    assert!(result.export_completed);
    assert!(result.closure_cancelled);
    assert!(result.audit_events >= 4);
    assert!(result.outbox_events >= 4);

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
