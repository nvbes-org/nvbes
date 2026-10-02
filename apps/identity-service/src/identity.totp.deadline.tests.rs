use super::*;
use crate::test_lock_waits::release_after_deadline;

#[tokio::test]
async fn totp_confirmation_and_step_up_never_outlive_the_session() {
    let db = isolated_database().await;
    let (session, token) = session(&db).await;
    let crypto = crypto();
    let pending = start(&db, &crypto, &token).await.unwrap();
    let code = code(&pending);
    let deadline: DateTime<Utc> = sqlx::query_scalar("UPDATE identity_sessions SET expires_at=clock_timestamp()+interval '1 minute' WHERE id=$1 RETURNING expires_at")
        .bind(session).fetch_one(&db).await.unwrap();
    assert_eq!(
        confirm(&db, &crypto, &token, pending.factor_id, &code)
            .await
            .unwrap(),
        deadline
    );
    // Permit this synthetic counter once more to exercise grant expiry independently.
    sqlx::query("UPDATE identity_auth_factors SET last_accepted_counter=NULL WHERE id=$1")
        .bind(pending.factor_id)
        .execute(&db)
        .await
        .unwrap();
    assert_eq!(
        crate::mfa::grant_step_up(&db, &crypto, &token, &code)
            .await
            .unwrap(),
        deadline
    );
}

async fn rejected_confirmation(expiry: &str) {
    let db = isolated_database().await;
    let (session, token) = session(&db).await;
    let crypto = crypto();
    let pending = start(&db, &crypto, &token).await.unwrap();
    let code = code(&pending);
    let (query, id) = match expiry {
        "factor" => (
            "UPDATE identity_auth_factors SET enrollment_expires_at=clock_timestamp()+interval '2 seconds' WHERE id=$1 RETURNING enrollment_expires_at",
            pending.factor_id,
        ),
        "session" => (
            "UPDATE identity_sessions SET expires_at=clock_timestamp()+interval '2 seconds' WHERE id=$1 RETURNING expires_at",
            session,
        ),
        _ => (
            "UPDATE identity_sessions SET authenticated_at=clock_timestamp()-interval '5 minutes'+interval '2 seconds' WHERE id=$1 RETURNING authenticated_at+interval '5 minutes'",
            session,
        ),
    };
    let deadline = sqlx::query_scalar(query)
        .bind(id)
        .fetch_one(&db)
        .await
        .unwrap();
    let mut blocker = db.begin().await.unwrap();
    sqlx::query("SELECT id FROM identity_auth_factors WHERE id=$1 FOR UPDATE")
        .bind(pending.factor_id)
        .fetch_one(&mut *blocker)
        .await
        .unwrap();
    let (result, ()) = tokio::join!(
        confirm(&db, &crypto, &token, pending.factor_id, &code),
        release_after_deadline(&db, blocker, deadline)
    );
    assert!(
        matches!(result, Err(TotpError::Invalid)),
        "expired {expiry} accepted: {}",
        result.is_ok()
    );
    let unchanged: bool = sqlx::query_scalar("SELECT state='pending' AND last_accepted_counter IS NULL AND enrollment_session_id IS NOT NULL FROM identity_auth_factors WHERE id=$1")
        .bind(pending.factor_id).fetch_one(&db).await.unwrap();
    assert!(unchanged);
    let grants: i64 =
        sqlx::query_scalar("SELECT count(*) FROM identity_sessions WHERE step_up_at IS NOT NULL")
            .fetch_one(&db)
            .await
            .unwrap();
    assert_eq!(grants, 0);
    let audits: i64 = sqlx::query_scalar("SELECT count(*) FROM identity_audit_events WHERE event_type IN ('identity.mfa_totp_enrolled','identity.step_up_granted')")
        .fetch_one(&db).await.unwrap();
    assert_eq!(audits, 0);
}

#[tokio::test]
async fn pending_expiry_during_factor_lock_cannot_confirm() {
    rejected_confirmation("factor").await;
}
#[tokio::test]
async fn session_expiry_during_factor_lock_cannot_confirm() {
    rejected_confirmation("session").await;
}
#[tokio::test]
async fn primary_expiry_during_factor_lock_cannot_confirm() {
    rejected_confirmation("primary").await;
}

#[tokio::test]
async fn session_expiry_during_factor_lock_cannot_grant_totp() {
    let db = isolated_database().await;
    let (session, token) = session(&db).await;
    let crypto = crypto();
    let pending = start(&db, &crypto, &token).await.unwrap();
    let code = code(&pending);
    confirm(&db, &crypto, &token, pending.factor_id, &code)
        .await
        .unwrap();
    sqlx::query("UPDATE identity_auth_factors SET last_accepted_counter=NULL WHERE id=$1")
        .bind(pending.factor_id)
        .execute(&db)
        .await
        .unwrap();
    let deadline = sqlx::query_scalar("UPDATE identity_sessions SET step_up_at=NULL,step_up_method=NULL,step_up_expires_at=NULL,expires_at=clock_timestamp()+interval '2 seconds' WHERE id=$1 RETURNING expires_at")
        .bind(session).fetch_one(&db).await.unwrap();
    let mut blocker = db.begin().await.unwrap();
    sqlx::query("SELECT id FROM identity_auth_factors WHERE id=$1 FOR UPDATE")
        .bind(pending.factor_id)
        .fetch_one(&mut *blocker)
        .await
        .unwrap();
    let (result, ()) = tokio::join!(
        crate::mfa::grant_step_up(&db, &crypto, &token, &code),
        release_after_deadline(&db, blocker, deadline)
    );
    assert!(result.is_err(), "expired session received TOTP step-up");
    let untouched: bool = sqlx::query_scalar(
        "SELECT last_accepted_counter IS NULL FROM identity_auth_factors WHERE id=$1",
    )
    .bind(pending.factor_id)
    .fetch_one(&db)
    .await
    .unwrap();
    assert!(untouched);
    let untouched: bool =
        sqlx::query_scalar("SELECT step_up_at IS NULL FROM identity_sessions WHERE id=$1")
            .bind(session)
            .fetch_one(&db)
            .await
            .unwrap();
    assert!(untouched);
}
