use super::*;
use crate::test_lock_waits::release_after_deadline;

async fn rejects_after_wait(expire_session: bool) {
    let mut f = Fixture::new().await;
    let (ceremony, response) = f.assertion().await;
    let before: serde_json::Value =
        sqlx::query_scalar("SELECT passkey FROM identity_webauthn_credentials WHERE id=$1")
            .bind(f.credential)
            .fetch_one(&f.db)
            .await
            .unwrap();
    let (query, id) = if expire_session {
        (
            "UPDATE identity_sessions SET expires_at=clock_timestamp()+interval '2 seconds' WHERE id=$1 RETURNING expires_at",
            f.session,
        )
    } else {
        (
            "UPDATE identity_webauthn_challenges SET expires_at=clock_timestamp()+interval '2 seconds' WHERE id=$1 RETURNING expires_at",
            ceremony,
        )
    };
    let deadline = sqlx::query_scalar(query)
        .bind(id)
        .fetch_one(&f.db)
        .await
        .unwrap();
    let mut blocker = f.db.begin().await.unwrap();
    sqlx::query("SELECT id FROM identity_webauthn_challenges WHERE id=$1 FOR UPDATE")
        .bind(ceremony)
        .fetch_one(&mut *blocker)
        .await
        .unwrap();
    let (result, ()) = tokio::join!(
        finish(&f.db, &f.server, &f.token, ceremony, &response),
        release_after_deadline(&f.db, blocker, deadline)
    );
    assert!(matches!(
        result,
        Err(WebauthnError::InvalidSession | WebauthnError::InvalidCeremony)
    ));
    let counts: (i64,i64,i64) = sqlx::query_as("SELECT (SELECT count(*) FROM identity_sessions WHERE step_up_at IS NOT NULL),(SELECT count(*) FROM identity_session_webauthn_credentials),(SELECT count(*) FROM identity_audit_events WHERE event_type='identity.step_up_granted')")
            .fetch_one(&f.db).await.unwrap();
    assert_eq!(counts, (0, 0, 0));
    let after: serde_json::Value =
        sqlx::query_scalar("SELECT passkey FROM identity_webauthn_credentials WHERE id=$1")
            .bind(f.credential)
            .fetch_one(&f.db)
            .await
            .unwrap();
    assert_eq!(before, after);
    let consumed: bool = sqlx::query_scalar(
        "SELECT consumed_at IS NOT NULL FROM identity_webauthn_challenges WHERE id=$1",
    )
    .bind(ceremony)
    .fetch_one(&f.db)
    .await
    .unwrap();
    assert!(!consumed);
}

#[tokio::test]
async fn challenge_expiry_after_lock_wait_cannot_grant_step_up() {
    rejects_after_wait(false).await;
}

#[tokio::test]
async fn session_expiry_after_lock_wait_cannot_grant_step_up() {
    rejects_after_wait(true).await;
}
