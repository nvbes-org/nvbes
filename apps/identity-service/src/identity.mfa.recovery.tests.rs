use super::*;
use crate::{mfa_crypto::MfaCrypto, test_fixtures, totp};
use nvbes_core::mfa::{current_counter, generate_totp_code};
use webauthn_authenticator_rs::{WebauthnAuthenticator, softpasskey::SoftPasskey};
use webauthn_rs::{Webauthn, prelude::*};

#[path = "identity.mfa.recovery.deadline.tests.rs"]
mod mfa_deadline_tests;

struct Fixture {
    db: PgPool,
    token: String,
    principal: Uuid,
    server: Webauthn,
}
impl Fixture {
    async fn new(keys: usize) -> Self {
        let db = test_fixtures::isolated_database().await;
        let (session, token) = test_fixtures::session(&db).await;
        let principal =
            sqlx::query_scalar("SELECT principal_id FROM identity_sessions WHERE id=$1")
                .bind(session)
                .fetch_one(&db)
                .await
                .unwrap();
        let crypto = MfaCrypto::with_rotation(1, [8; 32], None).unwrap();
        let pending = totp::start(&db, &crypto, &token).await.unwrap();
        totp::confirm(
            &db,
            &crypto,
            &token,
            pending.factor_id,
            &generate_totp_code(&pending.secret_base32, current_counter(Utc::now())),
        )
        .await
        .unwrap();
        let server =
            crate::webauthn::build_server("identity.example", "https://identity.example").unwrap();
        for _ in 0..keys {
            let started = crate::webauthn::registration::start(&db, &server, &token)
                .await
                .unwrap();
            let mut authenticator = WebauthnAuthenticator::new(SoftPasskey::new(true));
            let response = authenticator
                .do_registration("https://identity.example".parse().unwrap(), started.options)
                .unwrap();
            crate::webauthn::registration::finish(
                &db,
                &server,
                &token,
                started.ceremony_id,
                &response,
                "Previous key",
            )
            .await
            .unwrap();
        }
        Self {
            db,
            token,
            principal,
            server,
        }
    }
    async fn fresh_password_session(&self) -> String {
        let (session, token) = test_fixtures::session(&self.db).await;
        sqlx::query("UPDATE identity_sessions SET principal_id=$1 WHERE id=$2")
            .bind(self.principal)
            .bind(session)
            .execute(&self.db)
            .await
            .unwrap();
        token
    }
    async fn recovery(&self) -> RecoverySession {
        let codes = generate(&self.db, &self.token).await.unwrap();
        let token = self.fresh_password_session().await;
        redeem(&self.db, &token, &codes.codes[0]).await.unwrap()
    }
    async fn assertion(
        &self,
        token: &str,
    ) -> (
        Uuid,
        RegisterPublicKeyCredential,
        WebauthnAuthenticator<SoftPasskey>,
    ) {
        let started = registration::start(&self.db, &self.server, token)
            .await
            .unwrap();
        let mut authenticator = WebauthnAuthenticator::new(SoftPasskey::new(true));
        let response = authenticator
            .do_registration("https://identity.example".parse().unwrap(), started.options)
            .unwrap();
        (started.ceremony_id, response, authenticator)
    }
}

#[tokio::test]
async fn generation_requires_strong_proof_replaces_batch_and_never_persists_plaintext() {
    let f = Fixture::new(0).await;
    let password = f.fresh_password_session().await;
    assert!(matches!(
        generate(&f.db, &password).await,
        Err(RecoveryError::Invalid)
    ));
    let old = generate(&f.db, &f.token).await.unwrap();
    let new = generate(&f.db, &f.token).await.unwrap();
    assert_eq!(new.codes.len(), 10);
    assert_eq!(
        new.codes
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len(),
        10
    );
    assert!(
        new.codes
            .iter()
            .all(|c| c.len() == 48 && c.starts_with("nvr1_"))
    );
    let stored: Vec<Vec<u8>> = sqlx::query_scalar(
        "SELECT code_hash FROM identity_mfa_recovery_codes WHERE principal_id=$1",
    )
    .bind(f.principal)
    .fetch_all(&f.db)
    .await
    .unwrap();
    assert_eq!(stored.len(), 10);
    for code in &new.codes {
        assert!(stored.contains(&code_hash(f.principal, code)));
        assert!(!stored.contains(&code.as_bytes().to_vec()));
    }
    assert!(redeem(&f.db, &password, &old.codes[0]).await.is_err());
    let payloads: Vec<String> = sqlx::query_scalar("SELECT payload::text FROM identity_outbox")
        .fetch_all(&f.db)
        .await
        .unwrap();
    assert!(
        payloads
            .iter()
            .all(|p| !new.codes.iter().any(|c| p.contains(c)))
    );
}

#[tokio::test]
async fn concurrent_redemption_is_single_use_and_recovery_token_cannot_authorize_oauth() {
    let f = Fixture::new(0).await;
    let codes = generate(&f.db, &f.token).await.unwrap();
    let password = f.fresh_password_session().await;
    let (a, b) = tokio::join!(
        redeem(&f.db, &password, &codes.codes[0]),
        redeem(&f.db, &password, &codes.codes[0])
    );
    assert_ne!(a.is_ok(), b.is_ok());
    let recovery = a.or(b).unwrap();
    assert!(recovery.expires_at > Utc::now());
    assert!(generate(&f.db, &recovery.token).await.is_err());
    assert!(
        crate::webauthn::registration::start(&f.db, &f.server, &recovery.token)
            .await
            .is_err()
    );
    let clients = test_fixtures::clients();
    for token in [&recovery.token, &password, &f.token] {
        let handle = test_fixtures::request(
            &f.db,
            &clients,
            crate::oauth::store::RequestKind::Authorization,
        )
        .await;
        assert!(
            test_fixtures::authorize(&f.db, &clients, &handle, token)
                .await
                .is_err()
        );
    }
    let next = f.fresh_password_session().await;
    assert!(redeem(&f.db, &next, &codes.codes[0]).await.is_err());
    assert!(
        registration::start(&f.db, &f.server, &recovery.token)
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn foreign_code_stale_password_and_suspended_principal_are_refused() {
    let f = Fixture::new(0).await;
    let codes = generate(&f.db, &f.token).await.unwrap();
    let (_, other) = test_fixtures::session(&f.db).await;
    assert!(redeem(&f.db, &other, &codes.codes[0]).await.is_err());
    sqlx::query("UPDATE identity_sessions SET authenticated_at=clock_timestamp()-interval '6 minutes',step_up_at=NULL,step_up_expires_at=NULL,step_up_method=NULL WHERE token_hash=$1").bind(hash(&f.token)).execute(&f.db).await.unwrap();
    assert!(redeem(&f.db, &f.token, &codes.codes[0]).await.is_err());
    let password = f.fresh_password_session().await;
    sqlx::query("UPDATE identity_principals SET status='suspended' WHERE id=$1")
        .bind(f.principal)
        .execute(&f.db)
        .await
        .unwrap();
    assert!(redeem(&f.db, &password, &codes.codes[0]).await.is_err());
    let unused: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM identity_mfa_recovery_codes WHERE consumed_at IS NULL",
    )
    .fetch_one(&f.db)
    .await
    .unwrap();
    assert_eq!(unused, 10);
}

#[tokio::test]
async fn verified_replacement_at_capacity_retires_old_factors_and_requires_new_login() {
    let f = Fixture::new(10).await;
    let recovery = f.recovery().await;
    let (ceremony, response, mut authenticator) = f.assertion(&recovery.token).await;
    let intervening = f.fresh_password_session().await;
    let id = registration::finish(
        &f.db,
        &f.server,
        &recovery.token,
        ceremony,
        &response,
        "Recovered key",
    )
    .await
    .unwrap();
    assert!(
        registration::finish(
            &f.db,
            &f.server,
            &recovery.token,
            ceremony,
            &response,
            "Replay"
        )
        .await
        .is_err()
    );
    let live: Vec<Uuid> =
        sqlx::query_scalar("SELECT id FROM identity_webauthn_credentials WHERE revoked_at IS NULL")
            .fetch_all(&f.db)
            .await
            .unwrap();
    assert_eq!(live, vec![id]);
    let erased:bool=sqlx::query_scalar("SELECT state='revoked' AND octet_length(secret_ciphertext)=0 FROM identity_auth_factors WHERE principal_id=$1").bind(f.principal).fetch_one(&f.db).await.unwrap();
    assert!(erased);
    assert!(
        crate::totp::management::list(&f.db, &intervening)
            .await
            .is_err()
    );
    let codes: i64 = sqlx::query_scalar("SELECT count(*) FROM identity_mfa_recovery_codes")
        .fetch_one(&f.db)
        .await
        .unwrap();
    assert_eq!(codes, 0);
    // Prove that the recovered credential is usable cryptographically after a new login.
    let token = f.fresh_password_session().await;
    let started = crate::webauthn::step_up::start(&f.db, &f.server, &token)
        .await
        .unwrap();
    let assertion = authenticator
        .do_authentication("https://identity.example".parse().unwrap(), started.options)
        .unwrap();
    crate::webauthn::step_up::finish(&f.db, &f.server, &token, started.ceremony_id, &assertion)
        .await
        .unwrap();
    assert_eq!(generate(&f.db, &token).await.unwrap().codes.len(), 10);
}

#[tokio::test]
async fn notification_failure_rolls_back_redemption_and_preserves_code_and_sessions() {
    let f = Fixture::new(0).await;
    let codes = generate(&f.db, &f.token).await.unwrap();
    sqlx::query("ALTER TABLE identity_outbox ADD CONSTRAINT injected_recovery_failure CHECK(event_type<>'identity.mfa_recovery_started')").execute(&f.db).await.unwrap();
    assert!(matches!(
        redeem(&f.db, &f.token, &codes.codes[0]).await,
        Err(RecoveryError::Database(_))
    ));
    assert_eq!(
        crate::totp::management::list(&f.db, &f.token)
            .await
            .unwrap()
            .len(),
        1
    );
    let pending: i64 = sqlx::query_scalar("SELECT count(*) FROM identity_mfa_recovery_sessions")
        .fetch_one(&f.db)
        .await
        .unwrap();
    assert_eq!(pending, 0);
    sqlx::query("ALTER TABLE identity_outbox DROP CONSTRAINT injected_recovery_failure")
        .execute(&f.db)
        .await
        .unwrap();
    redeem(&f.db, &f.token, &codes.codes[0]).await.unwrap();
}

#[tokio::test]
async fn failed_completion_preserves_existing_factors_and_can_retry_same_attestation() {
    let f = Fixture::new(1).await;
    let recovery = f.recovery().await;
    let (ceremony, response, _) = f.assertion(&recovery.token).await;
    sqlx::query("ALTER TABLE identity_audit_events ADD CONSTRAINT injected_recovery_finish_failure CHECK(event_type<>'identity.mfa_recovered')").execute(&f.db).await.unwrap();
    assert!(matches!(
        registration::finish(
            &f.db,
            &f.server,
            &recovery.token,
            ceremony,
            &response,
            "New"
        )
        .await,
        Err(RecoveryError::Database(_))
    ));
    let active: bool = sqlx::query_scalar(
        "SELECT state='active' FROM identity_auth_factors WHERE principal_id=$1",
    )
    .bind(f.principal)
    .fetch_one(&f.db)
    .await
    .unwrap();
    assert!(active);
    sqlx::query(
        "ALTER TABLE identity_audit_events DROP CONSTRAINT injected_recovery_finish_failure",
    )
    .execute(&f.db)
    .await
    .unwrap();
    registration::finish(
        &f.db,
        &f.server,
        &recovery.token,
        ceremony,
        &response,
        "New",
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn expired_or_replaced_recovery_session_and_wrong_origin_cannot_replace_factors() {
    let f = Fixture::new(1).await;
    let codes = generate(&f.db, &f.token).await.unwrap();
    let recovery = redeem(&f.db, &f.token, &codes.codes[0]).await.unwrap();
    let (ceremony, response, _) = f.assertion(&recovery.token).await;
    let fresh = f.fresh_password_session().await;
    let replacement = redeem(&f.db, &fresh, &codes.codes[1]).await.unwrap();
    assert!(
        registration::finish(
            &f.db,
            &f.server,
            &recovery.token,
            ceremony,
            &response,
            "Old"
        )
        .await
        .is_err()
    );
    let started = registration::start(&f.db, &f.server, &replacement.token)
        .await
        .unwrap();
    let mut authenticator = WebauthnAuthenticator::new(SoftPasskey::new(true));
    let mut options = started.options;
    // Generate a real signature for a hostile origin with the authenticator's test RP changed.
    options.public_key.rp.id = "evil.example".into();
    let wrong = authenticator
        .do_registration("https://evil.example".parse().unwrap(), options)
        .unwrap();
    assert!(
        registration::finish(
            &f.db,
            &f.server,
            &replacement.token,
            started.ceremony_id,
            &wrong,
            "Wrong origin"
        )
        .await
        .is_err()
    );
    let (ceremony, response, _) = f.assertion(&replacement.token).await;
    sqlx::query("UPDATE identity_mfa_recovery_sessions SET expires_at=clock_timestamp() WHERE token_hash=$1").bind(hash(&replacement.token)).execute(&f.db).await.unwrap();
    assert!(
        registration::finish(
            &f.db,
            &f.server,
            &replacement.token,
            ceremony,
            &response,
            "Expired"
        )
        .await
        .is_err()
    );
}
