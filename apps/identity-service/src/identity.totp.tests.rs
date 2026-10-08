use super::*;
use crate::test_fixtures::{isolated_database, session};
use nvbes_core::mfa::{current_counter, generate_totp_code};

#[path = "identity.totp.deadline.tests.rs"]
mod mfa_deadline_tests;

fn crypto() -> MfaCrypto {
    MfaCrypto::with_rotation(1, [8; 32], None).unwrap()
}
fn code(enrollment: &Enrollment) -> String {
    generate_totp_code(&enrollment.secret_base32, current_counter(Utc::now()))
}

#[tokio::test]
async fn enrollment_encrypts_secret_confirms_once_and_rejects_step_up_replay() {
    let db = isolated_database().await;
    let (_, token) = session(&db).await;
    let crypto = crypto();
    let pending = start(&db, &crypto, &token).await.unwrap();
    assert!(pending.expires_at > Utc::now());
    assert!(
        pending
            .provisioning_uri
            .starts_with("otpauth://totp/nvbes:")
    );
    let encrypted: bool = sqlx::query_scalar("SELECT secret_ciphertext<>convert_to($2,'UTF8') AND enrollment_session_id IS NOT NULL FROM identity_auth_factors WHERE id=$1")
        .bind(pending.factor_id).bind(&pending.secret_base32).fetch_one(&db).await.unwrap();
    assert!(encrypted);
    let code = code(&pending);
    assert!(
        confirm(&db, &crypto, &token, pending.factor_id, &code)
            .await
            .unwrap()
            > Utc::now()
    );
    assert!(
        confirm(&db, &crypto, &token, pending.factor_id, &code)
            .await
            .is_err()
    );
    assert!(
        crate::mfa::grant_step_up(&db, &crypto, &token, &code)
            .await
            .is_err()
    );
    assert!(matches!(
        start(&db, &crypto, &token).await,
        Err(TotpError::AlreadyActive)
    ));
    let cleared: bool = sqlx::query_scalar("SELECT state='active' AND enrollment_session_id IS NULL AND enrollment_expires_at IS NULL FROM identity_auth_factors WHERE id=$1")
        .bind(pending.factor_id).fetch_one(&db).await.unwrap();
    assert!(cleared);
}

#[tokio::test]
async fn replacing_pending_enrollment_invalidates_old_id_and_session_binding() {
    let db = isolated_database().await;
    let (first_session, token) = session(&db).await;
    let crypto = crypto();
    let old = start(&db, &crypto, &token).await.unwrap();
    let pending = start(&db, &crypto, &token).await.unwrap();
    assert_ne!(old.factor_id, pending.factor_id);
    assert_ne!(old.secret_base32, pending.secret_base32);
    assert!(
        confirm(&db, &crypto, &token, old.factor_id, &code(&old))
            .await
            .is_err()
    );
    let other = crate::oauth::store::random_secret();
    sqlx::query("INSERT INTO identity_sessions(id,principal_id,token_hash,expires_at,authenticated_at,primary_amr) SELECT $1,principal_id,$2,expires_at,authenticated_at,primary_amr FROM identity_sessions WHERE id=$3")
        .bind(Uuid::new_v4()).bind(crate::auth::hash_token(&other)).bind(first_session).execute(&db).await.unwrap();
    assert!(
        confirm(&db, &crypto, &other, pending.factor_id, &code(&pending))
            .await
            .is_err()
    );
    assert!(
        confirm(&db, &crypto, &token, pending.factor_id, &code(&pending))
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn expired_ceremony_stale_or_revoked_session_and_suspended_owner_are_refused() {
    let db = isolated_database().await;
    let crypto = crypto();
    for mutation in [
        "UPDATE identity_auth_factors SET enrollment_expires_at=clock_timestamp() WHERE enrollment_session_id=$1",
        "UPDATE identity_sessions SET authenticated_at=clock_timestamp()-interval '6 minutes' WHERE id=$1",
        "UPDATE identity_sessions SET revoked_at=clock_timestamp() WHERE id=$1",
        "UPDATE identity_principals SET status='suspended' WHERE id=(SELECT principal_id FROM identity_sessions WHERE id=$1)",
    ] {
        let (id, token) = session(&db).await;
        let pending = start(&db, &crypto, &token).await.unwrap();
        sqlx::query(mutation).bind(id).execute(&db).await.unwrap();
        assert!(
            confirm(&db, &crypto, &token, pending.factor_id, &code(&pending))
                .await
                .is_err()
        );
    }
}

#[tokio::test]
async fn concurrent_confirmation_has_one_winner_and_one_enrollment_audit() {
    let db = isolated_database().await;
    let (_, token) = session(&db).await;
    let crypto = crypto();
    let pending = start(&db, &crypto, &token).await.unwrap();
    let code = code(&pending);
    let (first, second) = tokio::join!(
        confirm(&db, &crypto, &token, pending.factor_id, &code),
        confirm(&db, &crypto, &token, pending.factor_id, &code)
    );
    assert_ne!(first.is_ok(), second.is_ok());
    let audits: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM identity_audit_events WHERE event_type='identity.mfa_totp_enrolled'",
    )
    .fetch_one(&db)
    .await
    .unwrap();
    assert_eq!(audits, 1);
}

#[tokio::test]
async fn audit_failure_rolls_back_activation_counter_and_session_grant() {
    let db = isolated_database().await;
    let (session, token) = session(&db).await;
    let crypto = crypto();
    let pending = start(&db, &crypto, &token).await.unwrap();
    sqlx::raw_sql("CREATE FUNCTION fail_totp_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.event_type='identity.mfa_totp_enrolled' THEN RAISE EXCEPTION 'injected failure'; END IF; RETURN NEW; END $$; CREATE TRIGGER fail_totp_audit BEFORE INSERT ON identity_audit_events FOR EACH ROW EXECUTE FUNCTION fail_totp_audit();").execute(&db).await.unwrap();
    let code = code(&pending);
    assert!(matches!(
        confirm(&db, &crypto, &token, pending.factor_id, &code).await,
        Err(TotpError::Database(_))
    ));
    let unchanged: bool = sqlx::query_scalar("SELECT f.state='pending' AND f.last_accepted_counter IS NULL AND s.step_up_expires_at IS NULL FROM identity_auth_factors f JOIN identity_sessions s ON s.id=$2 WHERE f.id=$1")
        .bind(pending.factor_id).bind(session).fetch_one(&db).await.unwrap();
    assert!(unchanged);
    sqlx::query("DROP TRIGGER fail_totp_audit ON identity_audit_events")
        .execute(&db)
        .await
        .unwrap();
    confirm(&db, &crypto, &token, pending.factor_id, &code)
        .await
        .unwrap();
}

#[tokio::test]
async fn existing_strong_factor_requires_strong_authentication_before_enrollment() {
    let db = isolated_database().await;
    let (session, token) = session(&db).await;
    let crypto = crypto();
    // A second strong factor must not be enrollable using only a stolen password.
    sqlx::query("INSERT INTO identity_webauthn_credentials(id,principal_id,credential_id,public_key,label) SELECT $1,principal_id,$2,'{}','Existing key' FROM identity_sessions WHERE id=$3")
        .bind(Uuid::new_v4()).bind(vec![7_u8; 32]).bind(session).execute(&db).await.unwrap();
    assert!(matches!(
        start(&db, &crypto, &token).await,
        Err(TotpError::Invalid)
    ));
    sqlx::query("UPDATE identity_sessions SET step_up_method='webauthn',step_up_at=clock_timestamp(),step_up_expires_at=clock_timestamp()+interval '5 minutes' WHERE id=$1")
        .bind(session).execute(&db).await.unwrap();
    start(&db, &crypto, &token).await.unwrap();
}
