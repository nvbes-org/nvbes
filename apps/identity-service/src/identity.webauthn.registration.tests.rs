use super::*;
use crate::test_fixtures::{isolated_database, session};
use webauthn_authenticator_rs::{WebauthnAuthenticator, softpasskey::SoftPasskey};

fn server() -> Webauthn {
    super::super::build_server("identity.example", "https://identity.example").unwrap()
}

fn response(options: CreationChallengeResponse) -> RegisterPublicKeyCredential {
    WebauthnAuthenticator::new(SoftPasskey::new(true))
        .do_registration(
            reqwest::Url::parse("https://identity.example").unwrap(),
            options,
        )
        .unwrap()
}

#[tokio::test]
async fn enrollment_persists_a_usable_passkey_and_consumes_once() {
    let db = isolated_database().await;
    let (session_id, token) = session(&db).await;
    let server = server();
    let started = start(&db, &server, &token).await.unwrap();
    let options_json = serde_json::to_value(&started.options).unwrap();
    assert_eq!(
        options_json["publicKey"]["authenticatorSelection"]["residentKey"],
        "preferred"
    );
    assert_eq!(
        options_json["publicKey"]["authenticatorSelection"]["requireResidentKey"],
        false
    );
    assert_eq!(
        options_json["publicKey"]["authenticatorSelection"]["userVerification"],
        "required"
    );
    let stored: Vec<u8> =
        sqlx::query_scalar("SELECT challenge FROM identity_webauthn_challenges WHERE id=$1")
            .bind(started.ceremony_id)
            .fetch_one(&db)
            .await
            .unwrap();
    assert_eq!(
        stored.as_slice(),
        started.options.public_key.challenge.as_ref()
    );
    let mut authenticator = WebauthnAuthenticator::new(SoftPasskey::new(true));
    let response = authenticator
        .do_registration(
            reqwest::Url::parse("https://identity.example").unwrap(),
            started.options,
        )
        .unwrap();
    let id = finish(
        &db,
        &server,
        &token,
        started.ceremony_id,
        &response,
        "Test passkey",
    )
    .await
    .unwrap();
    let value: serde_json::Value =
        sqlx::query_scalar("SELECT passkey FROM identity_webauthn_credentials WHERE id=$1")
            .bind(id)
            .fetch_one(&db)
            .await
            .unwrap();
    let passkey: Passkey = serde_json::from_value(value).unwrap();
    let (options, state) = server.start_passkey_authentication(&[passkey]).unwrap();
    let assertion = authenticator
        .do_authentication(
            reqwest::Url::parse("https://identity.example").unwrap(),
            options,
        )
        .unwrap();
    assert!(
        server
            .finish_passkey_authentication(&assertion, &state)
            .unwrap()
            .user_verified()
    );
    assert!(
        finish(
            &db,
            &server,
            &token,
            started.ceremony_id,
            &response,
            "Replay"
        )
        .await
        .is_err()
    );
    assert!(matches!(
        start(&db, &server, &token).await,
        Err(WebauthnError::InvalidSession)
    ));
    sqlx::query("UPDATE identity_sessions SET step_up_method='webauthn',step_up_at=clock_timestamp(),step_up_expires_at=clock_timestamp()+interval '5 minutes' WHERE id=$1")
        .bind(session_id).execute(&db).await.unwrap();
    let next = start(&db, &server, &token).await.unwrap();
    assert_eq!(
        next.options.public_key.exclude_credentials.unwrap().len(),
        1
    );
}

#[tokio::test]
async fn wrong_session_expired_ceremony_and_revoked_session_are_rejected() {
    let db = isolated_database().await;
    let (session_id, token) = session(&db).await;
    let (_, other) = session(&db).await;
    let server = server();
    let started = start(&db, &server, &token).await.unwrap();
    let response = response(started.options);
    assert!(
        finish(
            &db,
            &server,
            &other,
            started.ceremony_id,
            &response,
            "Wrong"
        )
        .await
        .is_err()
    );
    sqlx::query("UPDATE identity_webauthn_challenges SET expires_at=clock_timestamp() WHERE id=$1")
        .bind(started.ceremony_id)
        .execute(&db)
        .await
        .unwrap();
    assert!(
        finish(
            &db,
            &server,
            &token,
            started.ceremony_id,
            &response,
            "Expired"
        )
        .await
        .is_err()
    );
    sqlx::query("UPDATE identity_sessions SET revoked_at=clock_timestamp() WHERE id=$1")
        .bind(session_id)
        .execute(&db)
        .await
        .unwrap();
    assert!(matches!(
        start(&db, &server, &token).await,
        Err(WebauthnError::InvalidSession)
    ));
}

#[tokio::test]
async fn fresh_start_replaces_prior_state_and_concurrent_finish_has_one_winner() {
    let db = isolated_database().await;
    let (_, token) = session(&db).await;
    let server = server();
    let old = start(&db, &server, &token).await.unwrap();
    let old_response = response(old.options);
    let new = start(&db, &server, &token).await.unwrap();
    assert!(
        finish(&db, &server, &token, old.ceremony_id, &old_response, "Old")
            .await
            .is_err()
    );
    assert!(
        finish(
            &db,
            &server,
            &token,
            new.ceremony_id,
            &old_response,
            "Substitution"
        )
        .await
        .is_err()
    );
    let response = response(new.options);
    let (a, b) = tokio::join!(
        finish(&db, &server, &token, new.ceremony_id, &response, "First"),
        finish(&db, &server, &token, new.ceremony_id, &response, "Second")
    );
    assert_ne!(a.is_ok(), b.is_ok());
}

#[tokio::test]
async fn credential_write_failure_preserves_challenge_for_retry() {
    let db = isolated_database().await;
    let (_, token) = session(&db).await;
    let server = server();
    let started = start(&db, &server, &token).await.unwrap();
    let response = response(started.options);
    sqlx::query("ALTER TABLE identity_webauthn_credentials ADD CONSTRAINT injected_failure CHECK (label <> 'Fail')")
        .execute(&db).await.unwrap();
    assert!(matches!(
        finish(&db, &server, &token, started.ceremony_id, &response, "Fail").await,
        Err(WebauthnError::Database(_))
    ));
    assert!(
        finish(
            &db,
            &server,
            &token,
            started.ceremony_id,
            &response,
            "Retry"
        )
        .await
        .is_ok()
    );
}

#[tokio::test]
async fn wrong_origin_and_missing_user_verification_cannot_enroll() {
    let db = isolated_database().await;
    let (_, token) = session(&db).await;
    let server = server();
    let started = start(&db, &server, &token).await.unwrap();
    let wrong_origin = WebauthnAuthenticator::new(SoftPasskey::new(true))
        .do_registration(
            reqwest::Url::parse("https://evil.identity.example").unwrap(),
            started.options.clone(),
        )
        .unwrap();
    assert!(matches!(
        finish(
            &db,
            &server,
            &token,
            started.ceremony_id,
            &wrong_origin,
            "Wrong origin"
        )
        .await,
        Err(WebauthnError::InvalidCeremony)
    ));
    // A malicious client can edit browser options, but cannot weaken the
    // verification policy retained in server-side ceremony state.
    let mut altered = serde_json::to_value(&started.options).unwrap();
    altered["publicKey"]["authenticatorSelection"]["userVerification"] = "discouraged".into();
    let missing_uv = response(serde_json::from_value(altered).unwrap());
    assert!(matches!(
        finish(
            &db,
            &server,
            &token,
            started.ceremony_id,
            &missing_uv,
            "Missing UV"
        )
        .await,
        Err(WebauthnError::InvalidCeremony)
    ));
    let valid = response(started.options);
    assert!(
        finish(&db, &server, &token, started.ceremony_id, &valid, "Valid")
            .await
            .is_ok()
    );
}
