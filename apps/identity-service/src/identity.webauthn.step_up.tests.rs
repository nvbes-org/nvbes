use super::*;
use crate::{
    test_fixtures::{isolated_database, session},
    webauthn::registration,
};
use webauthn_authenticator_rs::{WebauthnAuthenticator, softpasskey::SoftPasskey};

struct Fixture {
    db: PgPool,
    server: Webauthn,
    token: String,
    session: Uuid,
    credential: Uuid,
    authenticator: WebauthnAuthenticator<SoftPasskey>,
}

impl Fixture {
    async fn new() -> Self {
        let db = isolated_database().await;
        let (session, token) = session(&db).await;
        let server =
            crate::webauthn::build_server("identity.example", "https://identity.example").unwrap();
        let mut authenticator = WebauthnAuthenticator::new(SoftPasskey::new(true));
        let started = registration::start(&db, &server, &token).await.unwrap();
        let response = authenticator
            .do_registration("https://identity.example".parse().unwrap(), started.options)
            .unwrap();
        let credential =
            registration::finish(&db, &server, &token, started.ceremony_id, &response, "Test")
                .await
                .unwrap();
        Self {
            db,
            server,
            token,
            session,
            credential,
            authenticator,
        }
    }

    async fn assertion(&mut self) -> (Uuid, PublicKeyCredential) {
        let started = start(&self.db, &self.server, &self.token).await.unwrap();
        let response = self
            .authenticator
            .do_authentication("https://identity.example".parse().unwrap(), started.options)
            .unwrap();
        (started.ceremony_id, response)
    }
}

#[tokio::test]
async fn assertion_grants_fresh_step_up_and_cannot_be_replayed() {
    let mut f = Fixture::new().await;
    let (ceremony, response) = f.assertion().await;
    let expires = finish(&f.db, &f.server, &f.token, ceremony, &response)
        .await
        .unwrap();
    assert!(expires > Utc::now());
    let stored: (String, bool) = sqlx::query_as(
        "SELECT step_up_method,step_up_at IS NOT NULL FROM identity_sessions WHERE id=$1",
    )
    .bind(f.session)
    .fetch_one(&f.db)
    .await
    .unwrap();
    assert_eq!(stored, ("webauthn".into(), true));
    assert!(
        finish(&f.db, &f.server, &f.token, ceremony, &response)
            .await
            .is_err()
    );
    assert!(
        registration::start(&f.db, &f.server, &f.token)
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn concurrent_assertion_grants_once() {
    let mut f = Fixture::new().await;
    let (ceremony, response) = f.assertion().await;
    let (a, b) = tokio::join!(
        finish(&f.db, &f.server, &f.token, ceremony, &response),
        finish(&f.db, &f.server, &f.token, ceremony, &response)
    );
    assert_ne!(a.is_ok(), b.is_ok());
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM identity_audit_events WHERE event_type='identity.step_up_granted'",
    )
    .fetch_one(&f.db)
    .await
    .unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn revoked_credential_session_and_wrong_session_reject_pending_assertions() {
    let mut f = Fixture::new().await;
    let (ceremony, response) = f.assertion().await;
    let (_, other) = session(&f.db).await;
    assert!(
        finish(&f.db, &f.server, &other, ceremony, &response)
            .await
            .is_err()
    );
    sqlx::query(
        "UPDATE identity_webauthn_credentials SET revoked_at=clock_timestamp() WHERE id=$1",
    )
    .bind(f.credential)
    .execute(&f.db)
    .await
    .unwrap();
    assert!(
        finish(&f.db, &f.server, &f.token, ceremony, &response)
            .await
            .is_err()
    );
    assert!(start(&f.db, &f.server, &f.token).await.is_err());
    sqlx::query("UPDATE identity_sessions SET revoked_at=clock_timestamp() WHERE id=$1")
        .bind(f.session)
        .execute(&f.db)
        .await
        .unwrap();
    assert!(matches!(
        finish(&f.db, &f.server, &f.token, ceremony, &response).await,
        Err(WebauthnError::InvalidSession)
    ));
}

#[tokio::test]
async fn failed_session_write_rolls_back_credential_challenge_and_audit() {
    let mut f = Fixture::new().await;
    let (ceremony, response) = f.assertion().await;
    let before: serde_json::Value =
        sqlx::query_scalar("SELECT passkey FROM identity_webauthn_credentials WHERE id=$1")
            .bind(f.credential)
            .fetch_one(&f.db)
            .await
            .unwrap();
    sqlx::query("ALTER TABLE identity_sessions ADD CONSTRAINT injected_stepup_failure CHECK (step_up_method IS NULL)")
        .execute(&f.db).await.unwrap();
    assert!(matches!(
        finish(&f.db, &f.server, &f.token, ceremony, &response).await,
        Err(WebauthnError::Database(_))
    ));
    let after: serde_json::Value =
        sqlx::query_scalar("SELECT passkey FROM identity_webauthn_credentials WHERE id=$1")
            .bind(f.credential)
            .fetch_one(&f.db)
            .await
            .unwrap();
    assert_eq!(before, after);
    sqlx::query("ALTER TABLE identity_sessions DROP CONSTRAINT injected_stepup_failure")
        .execute(&f.db)
        .await
        .unwrap();
    assert!(
        finish(&f.db, &f.server, &f.token, ceremony, &response)
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn later_counter_from_another_session_invalidates_an_older_pending_assertion() {
    let mut f = Fixture::new().await;
    let (second_id, second_token) = session(&f.db).await;
    sqlx::query("UPDATE identity_sessions SET principal_id=(SELECT principal_id FROM identity_sessions WHERE id=$1) WHERE id=$2")
        .bind(f.session).bind(second_id).execute(&f.db).await.unwrap();
    let first = start(&f.db, &f.server, &f.token).await.unwrap();
    let second = start(&f.db, &f.server, &second_token).await.unwrap();
    let first_response = f
        .authenticator
        .do_authentication("https://identity.example".parse().unwrap(), first.options)
        .unwrap();
    let second_response = f
        .authenticator
        .do_authentication("https://identity.example".parse().unwrap(), second.options)
        .unwrap();
    finish(
        &f.db,
        &f.server,
        &second_token,
        second.ceremony_id,
        &second_response,
    )
    .await
    .unwrap();
    assert!(matches!(
        finish(
            &f.db,
            &f.server,
            &f.token,
            first.ceremony_id,
            &first_response
        )
        .await,
        Err(WebauthnError::InvalidCeremony)
    ));
    let granted: bool =
        sqlx::query_scalar("SELECT step_up_at IS NOT NULL FROM identity_sessions WHERE id=$1")
            .bind(f.session)
            .fetch_one(&f.db)
            .await
            .unwrap();
    assert!(!granted);
}

#[tokio::test]
async fn altered_assertion_and_expired_state_never_grant_step_up() {
    let mut f = Fixture::new().await;
    let (ceremony, response) = f.assertion().await;
    let mut altered = serde_json::to_value(&response).unwrap();
    altered["response"]["signature"] = "AA".into();
    let altered: PublicKeyCredential = serde_json::from_value(altered).unwrap();
    assert!(
        finish(&f.db, &f.server, &f.token, ceremony, &altered)
            .await
            .is_err()
    );
    sqlx::query("UPDATE identity_webauthn_challenges SET expires_at=clock_timestamp() WHERE id=$1")
        .bind(ceremony)
        .execute(&f.db)
        .await
        .unwrap();
    assert!(
        finish(&f.db, &f.server, &f.token, ceremony, &response)
            .await
            .is_err()
    );
    let granted: bool =
        sqlx::query_scalar("SELECT step_up_at IS NOT NULL FROM identity_sessions WHERE id=$1")
            .bind(f.session)
            .fetch_one(&f.db)
            .await
            .unwrap();
    assert!(!granted);
}
