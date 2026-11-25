use super::*;
use crate::test_lock_waits::release_after_deadline;

async fn rejects_after_wait(expiry: &str) {
    let db = isolated_database().await;
    let (session, token) = session(&db).await;
    let server = server();
    if expiry == "strong" {
        let first = start(&db, &server, &token).await.unwrap();
        let first_response = response(first.options);
        finish(
            &db,
            &server,
            &token,
            first.ceremony_id,
            &first_response,
            "Existing key",
        )
        .await
        .unwrap();
        sqlx::query("UPDATE identity_sessions SET step_up_method='webauthn',step_up_at=clock_timestamp(),step_up_expires_at=clock_timestamp()+interval '5 minutes' WHERE id=$1")
                .bind(session).execute(&db).await.unwrap();
    }
    let started = start(&db, &server, &token).await.unwrap();
    let response = response(started.options);
    let (query, id) = match expiry {
        "challenge" => (
            "UPDATE identity_webauthn_challenges SET expires_at=clock_timestamp()+interval '2 seconds' WHERE id=$1 RETURNING expires_at",
            started.ceremony_id,
        ),
        "session" => (
            "UPDATE identity_sessions SET expires_at=clock_timestamp()+interval '2 seconds' WHERE id=$1 RETURNING expires_at",
            session,
        ),
        "primary" => (
            "UPDATE identity_sessions SET authenticated_at=clock_timestamp()-interval '5 minutes'+interval '2 seconds' WHERE id=$1 RETURNING authenticated_at+interval '5 minutes'",
            session,
        ),
        _ => (
            "UPDATE identity_sessions SET step_up_expires_at=clock_timestamp()+interval '2 seconds' WHERE id=$1 RETURNING step_up_expires_at",
            session,
        ),
    };
    let deadline = sqlx::query_scalar(query)
        .bind(id)
        .fetch_one(&db)
        .await
        .unwrap();
    let mut blocker = db.begin().await.unwrap();
    sqlx::query("SELECT id FROM identity_webauthn_challenges WHERE id=$1 FOR UPDATE")
        .bind(started.ceremony_id)
        .fetch_one(&mut *blocker)
        .await
        .unwrap();
    let (result, ()) = tokio::join!(
        finish(
            &db,
            &server,
            &token,
            started.ceremony_id,
            &response,
            "Expired enrollment"
        ),
        release_after_deadline(&db, blocker, deadline)
    );
    assert!(
        matches!(
            result,
            Err(WebauthnError::InvalidSession | WebauthnError::InvalidCeremony)
        ),
        "{expiry}"
    );
    let counts: (i64,i64) = sqlx::query_as("SELECT (SELECT count(*) FROM identity_webauthn_credentials),(SELECT count(*) FROM identity_audit_events WHERE event_type='identity.webauthn.enrolled')")
            .fetch_one(&db).await.unwrap();
    let existing = if expiry == "strong" { 1 } else { 0 };
    assert_eq!(counts, (existing, existing));
    let consumed: bool = sqlx::query_scalar(
        "SELECT consumed_at IS NOT NULL FROM identity_webauthn_challenges WHERE id=$1",
    )
    .bind(started.ceremony_id)
    .fetch_one(&db)
    .await
    .unwrap();
    assert!(!consumed);
}

#[tokio::test]
async fn challenge_expiry_after_lock_wait_cannot_enroll() {
    rejects_after_wait("challenge").await;
}

#[tokio::test]
async fn session_expiry_after_lock_wait_cannot_enroll() {
    rejects_after_wait("session").await;
}

#[tokio::test]
async fn primary_expiry_after_lock_wait_cannot_enroll() {
    rejects_after_wait("primary").await;
}

#[tokio::test]
async fn strong_expiry_after_lock_wait_cannot_enroll() {
    rejects_after_wait("strong").await;
}
