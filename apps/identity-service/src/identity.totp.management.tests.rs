use super::*;
use crate::{
    mfa_crypto::MfaCrypto,
    test_fixtures::{isolated_database, session},
    totp,
};
use nvbes_core::mfa::{current_counter, generate_totp_code};
use webauthn_authenticator_rs::{WebauthnAuthenticator, softpasskey::SoftPasskey};

#[path = "identity.mfa.rotation.rs"]
mod rotation;

struct Fixture {
    db: PgPool,
    token: String,
    session: Uuid,
    factor: Uuid,
    passkey: Option<Uuid>,
}
impl Fixture {
    async fn new(with_passkey: bool) -> Self {
        let db = isolated_database().await;
        let (session, token) = session(&db).await;
        let crypto = MfaCrypto::with_rotation(1, [8; 32], None).unwrap();
        let enrollment = totp::start(&db, &crypto, &token).await.unwrap();
        let code = generate_totp_code(&enrollment.secret_base32, current_counter(Utc::now()));
        totp::confirm(&db, &crypto, &token, enrollment.factor_id, &code)
            .await
            .unwrap();
        let passkey = if with_passkey {
            let server =
                crate::webauthn::build_server("identity.example", "https://identity.example")
                    .unwrap();
            let started = crate::webauthn::registration::start(&db, &server, &token)
                .await
                .unwrap();
            let mut authenticator = WebauthnAuthenticator::new(SoftPasskey::new(true));
            let response = authenticator
                .do_registration("https://identity.example".parse().unwrap(), started.options)
                .unwrap();
            Some(
                crate::webauthn::registration::finish(
                    &db,
                    &server,
                    &token,
                    started.ceremony_id,
                    &response,
                    "Backup",
                )
                .await
                .unwrap(),
            )
        } else {
            None
        };
        Self {
            db,
            token,
            session,
            factor: enrollment.factor_id,
            passkey,
        }
    }
}

#[tokio::test]
async fn owned_metadata_excludes_secrets_and_revocation_closes_all_sessions() {
    let f = Fixture::new(true).await;
    let rows = list(&f.db, &f.token).await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, f.factor);
    assert_eq!(
        serde_json::to_value(&rows[0])
            .unwrap()
            .as_object()
            .unwrap()
            .len(),
        2
    );
    let (_, other) = session(&f.db).await;
    assert!(list(&f.db, &other).await.unwrap().is_empty());
    assert!(matches!(
        revoke(&f.db, &other, f.factor).await,
        Err(TotpError::Invalid)
    ));
    sqlx::query("INSERT INTO identity_sessions(id,principal_id,token_hash,expires_at,authenticated_at,primary_amr) SELECT $1,principal_id,$2,expires_at,authenticated_at,'pwd' FROM identity_sessions WHERE id=$3")
        .bind(Uuid::new_v4()).bind(crate::auth::hash_token("another-session")).bind(f.session).execute(&f.db).await.unwrap();
    revoke(&f.db, &f.token, f.factor).await.unwrap();
    let remaining: i64 = sqlx::query_scalar("SELECT count(*) FROM identity_sessions WHERE principal_id=(SELECT principal_id FROM identity_sessions WHERE id=$1) AND revoked_at IS NULL")
        .bind(f.session).fetch_one(&f.db).await.unwrap();
    assert_eq!(remaining, 0);
    let erased: bool = sqlx::query_scalar("SELECT state='revoked' AND octet_length(secret_ciphertext)=0 FROM identity_auth_factors WHERE id=$1").bind(f.factor).fetch_one(&f.db).await.unwrap();
    assert!(erased);
    assert!(matches!(
        list(&f.db, &f.token).await,
        Err(TotpError::Invalid)
    ));
    assert!(list(&f.db, &other).await.is_ok());
    // An erased old-version factor must not block rotating unrelated secrets.
    let old = MfaCrypto::with_rotation(1, [8; 32], None).unwrap();
    let pending = totp::start(&f.db, &old, &other).await.unwrap();
    let rotated = MfaCrypto::with_rotation(2, [9; 32], Some((1, [8; 32]))).unwrap();
    assert_eq!(rotation::rotate(&f.db, &rotated).await.unwrap(), 1);
    let code = generate_totp_code(&pending.secret_base32, current_counter(Utc::now()));
    totp::confirm(&f.db, &rotated, &other, pending.factor_id, &code)
        .await
        .unwrap();
}

#[tokio::test]
async fn last_factor_and_stale_step_up_are_refused_without_mutation() {
    let f = Fixture::new(false).await;
    assert!(matches!(
        revoke(&f.db, &f.token, f.factor).await,
        Err(TotpError::LastFactor)
    ));
    sqlx::query("UPDATE identity_sessions SET step_up_at=clock_timestamp()-interval '6 minutes' WHERE id=$1").bind(f.session).execute(&f.db).await.unwrap();
    assert!(matches!(
        revoke(&f.db, &f.token, f.factor).await,
        Err(TotpError::Invalid)
    ));
    assert_eq!(list(&f.db, &f.token).await.unwrap().len(), 1);
    sqlx::query("UPDATE identity_principals SET status='suspended' WHERE id=(SELECT principal_id FROM identity_sessions WHERE id=$1)").bind(f.session).execute(&f.db).await.unwrap();
    assert!(matches!(
        list(&f.db, &f.token).await,
        Err(TotpError::Invalid)
    ));
}

#[tokio::test]
async fn competing_totp_and_passkey_revocations_preserve_one_strong_factor() {
    let f = Fixture::new(true).await;
    let (totp, webauthn) = tokio::join!(
        revoke(&f.db, &f.token, f.factor),
        crate::webauthn::credentials::revoke(&f.db, &f.token, f.passkey.unwrap())
    );
    assert_ne!(totp.is_ok(), webauthn.is_ok());
    let count: i64 = sqlx::query_scalar("SELECT (SELECT count(*) FROM identity_auth_factors WHERE state='active') + (SELECT count(*) FROM identity_webauthn_credentials WHERE revoked_at IS NULL)")
        .fetch_one(&f.db).await.unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn audit_failure_rolls_back_secret_erasure_and_session_revocation() {
    let f = Fixture::new(true).await;
    sqlx::query("ALTER TABLE identity_audit_events ADD CONSTRAINT injected_totp_revoke_failure CHECK(event_type<>'identity.mfa_totp_revoked')").execute(&f.db).await.unwrap();
    assert!(matches!(
        revoke(&f.db, &f.token, f.factor).await,
        Err(TotpError::Database(_))
    ));
    assert_eq!(list(&f.db, &f.token).await.unwrap().len(), 1);
    let kept: bool = sqlx::query_scalar(
        "SELECT octet_length(secret_ciphertext)>0 FROM identity_auth_factors WHERE id=$1",
    )
    .bind(f.factor)
    .fetch_one(&f.db)
    .await
    .unwrap();
    assert!(kept);
    sqlx::query("ALTER TABLE identity_audit_events DROP CONSTRAINT injected_totp_revoke_failure")
        .execute(&f.db)
        .await
        .unwrap();
    revoke(&f.db, &f.token, f.factor).await.unwrap();
}
