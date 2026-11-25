use super::*;
use crate::test_lock_waits::release_after_deadline;

async fn rejects_replacement(expire_session: bool) {
    let f = Fixture::new(1).await;
    let recovery = f.recovery().await;
    let (ceremony, response, _) = f.assertion(&recovery.token).await;
    let query = if expire_session {
        "UPDATE identity_mfa_recovery_sessions SET expires_at=clock_timestamp()+interval '2 seconds' WHERE principal_id=$1 RETURNING expires_at"
    } else {
        "UPDATE identity_webauthn_challenges SET expires_at=clock_timestamp()+interval '2 seconds' WHERE principal_id=$1 AND recovery_session_id IS NOT NULL RETURNING expires_at"
    };
    let deadline = sqlx::query_scalar(query)
        .bind(f.principal)
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
        registration::finish(
            &f.db,
            &f.server,
            &recovery.token,
            ceremony,
            &response,
            "Expired replacement"
        ),
        release_after_deadline(&f.db, blocker, deadline)
    );
    assert!(
        matches!(result, Err(RecoveryError::Invalid)),
        "expired recovery accepted: {}",
        result.is_ok()
    );
    let counts: (i64,i64,i64,i64) = sqlx::query_as("SELECT (SELECT count(*) FROM identity_webauthn_credentials WHERE revoked_at IS NULL),(SELECT count(*) FROM identity_auth_factors WHERE state='active'),(SELECT count(*) FROM identity_mfa_recovery_sessions),(SELECT count(*) FROM identity_mfa_recovery_codes)")
        .fetch_one(&f.db).await.unwrap();
    assert_eq!(counts, (1, 1, 1, 10));
    let notices: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM identity_outbox WHERE event_type='identity.mfa_recovered'",
    )
    .fetch_one(&f.db)
    .await
    .unwrap();
    assert_eq!(notices, 0);
}

#[tokio::test]
async fn recovery_challenge_expiry_during_lock_cannot_replace_factors() {
    rejects_replacement(false).await;
}
#[tokio::test]
async fn recovery_session_expiry_during_lock_cannot_replace_factors() {
    rejects_replacement(true).await;
}

#[tokio::test]
async fn primary_expiry_during_code_lock_cannot_redeem() {
    let f = Fixture::new(0).await;
    let codes = generate(&f.db, &f.token).await.unwrap();
    let password = f.fresh_password_session().await;
    let deadline = sqlx::query_scalar("UPDATE identity_sessions SET authenticated_at=clock_timestamp()-interval '5 minutes'+interval '2 seconds' WHERE token_hash=$1 RETURNING authenticated_at+interval '5 minutes'")
        .bind(hash(&password)).fetch_one(&f.db).await.unwrap();
    let mut blocker = f.db.begin().await.unwrap();
    sqlx::query(
        "SELECT principal_id FROM identity_mfa_recovery_codes WHERE principal_id=$1 FOR UPDATE",
    )
    .bind(f.principal)
    .fetch_all(&mut *blocker)
    .await
    .unwrap();
    let (result, ()) = tokio::join!(
        redeem(&f.db, &password, &codes.codes[0]),
        release_after_deadline(&f.db, blocker, deadline)
    );
    assert!(
        matches!(result, Err(RecoveryError::Invalid)),
        "stale primary accepted: {}",
        result.is_ok()
    );
    let counts:(i64,i64,i64) = sqlx::query_as("SELECT (SELECT count(*) FROM identity_mfa_recovery_codes WHERE consumed_at IS NOT NULL),(SELECT count(*) FROM identity_mfa_recovery_sessions),(SELECT count(*) FROM identity_sessions WHERE revoked_at IS NOT NULL)")
        .fetch_one(&f.db).await.unwrap();
    assert_eq!(counts, (0, 0, 0));
}

#[tokio::test]
async fn strong_expiry_during_code_lock_cannot_replace_recovery_codes() {
    let f = Fixture::new(0).await;
    let before = generate(&f.db, &f.token).await.unwrap();
    let deadline = sqlx::query_scalar("UPDATE identity_sessions SET step_up_expires_at=clock_timestamp()+interval '2 seconds' WHERE token_hash=$1 RETURNING step_up_expires_at")
        .bind(hash(&f.token)).fetch_one(&f.db).await.unwrap();
    let mut blocker = f.db.begin().await.unwrap();
    sqlx::query(
        "SELECT principal_id FROM identity_mfa_recovery_codes WHERE principal_id=$1 FOR UPDATE",
    )
    .bind(f.principal)
    .fetch_all(&mut *blocker)
    .await
    .unwrap();
    let (result, ()) = tokio::join!(
        generate(&f.db, &f.token),
        release_after_deadline(&f.db, blocker, deadline)
    );
    assert!(
        matches!(result, Err(RecoveryError::Invalid)),
        "stale strong proof accepted: {}",
        result.is_ok()
    );
    let hashes: Vec<Vec<u8>> =
        sqlx::query_scalar("SELECT code_hash FROM identity_mfa_recovery_codes")
            .fetch_all(&f.db)
            .await
            .unwrap();
    assert_eq!(hashes.len(), 10);
    assert!(
        before
            .codes
            .iter()
            .all(|code| hashes.contains(&code_hash(f.principal, code)))
    );
}
