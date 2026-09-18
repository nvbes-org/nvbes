use super::*;
use crate::test_lock_waits::release_after_deadline;

#[tokio::test]
async fn login_challenge_expiring_while_waiting_for_principal_rolls_back_every_effect() {
    let mut f = Fixture::new().await;
    let (ceremony, response) = f.assertion().await;
    let before: serde_json::Value =
        sqlx::query_scalar("SELECT passkey FROM identity_webauthn_credentials WHERE id=$1")
            .bind(f.credential)
            .fetch_one(&f.db)
            .await
            .unwrap();
    let deadline = sqlx::query_scalar("UPDATE identity_webauthn_challenges SET expires_at=clock_timestamp()+interval '2 seconds' WHERE id=$1 RETURNING expires_at")
        .bind(ceremony).fetch_one(&f.db).await.unwrap();
    let mut blocker = f.db.begin().await.unwrap();
    sqlx::query("SELECT id FROM identity_principals WHERE id=$1 FOR UPDATE")
        .bind(f.principal)
        .fetch_one(&mut *blocker)
        .await
        .unwrap();
    let (result, ()) = tokio::join!(
        f.finish(ceremony, &response),
        release_after_deadline(&f.db, blocker, deadline)
    );
    assert!(matches!(
        result,
        Err(StoreError::Protocol(OAuthError::InvalidRequest))
    ));
    let counts: (i64,i64,i64,i64) = sqlx::query_as("SELECT (SELECT count(*) FROM identity_sessions WHERE primary_amr='webauthn'),(SELECT count(*) FROM identity_session_webauthn_credentials),(SELECT count(*) FROM identity_audit_events WHERE event_type='identity.session.passkey_authenticated'),(SELECT count(*) FROM identity_oauth_requests WHERE bound_session_id IS NOT NULL)")
        .fetch_one(&f.db).await.unwrap();
    assert_eq!(counts, (0, 0, 0, 0));
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
