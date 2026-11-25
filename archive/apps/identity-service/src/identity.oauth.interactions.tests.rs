use super::*;
use crate::{
    oauth::{
        consent,
        store::{self, RequestKind},
    },
    test_fixtures::{browser_proof, clients, database, input, request, session},
};

#[tokio::test]
async fn consent_is_bound_to_browser_csrf_and_the_displayed_session() {
    let db = database().await;
    let clients = clients();
    let handle = request(&db, &clients, RequestKind::Authorization).await;
    let (_, token) = session(&db).await;
    let (_, other_session) = session(&db).await;
    let browser = random_secret();
    let started = begin(&db, &clients, &handle, &browser, Some(&token))
        .await
        .unwrap();
    assert!(!started.needs_login);
    assert!(
        begin(&db, &clients, &handle, &random_secret(), Some(&token))
            .await
            .is_err()
    );
    for proof in [
        browser_proof(&random_secret(), &started.csrf_token, Some(&token)),
        browser_proof(&browser, &random_secret(), Some(&token)),
        browser_proof(&browser, &started.csrf_token, Some(&other_session)),
        browser_proof(&browser, &started.csrf_token, None),
    ] {
        assert!(
            consent::approve(&db, &clients, &handle, &proof)
                .await
                .is_err()
        );
    }
    assert!(store::load_request(&db, &clients, &handle).await.is_ok());
    let proof = browser_proof(&browser, &started.csrf_token, Some(&token));
    let (a, b) = tokio::join!(
        consent::approve(&db, &clients, &handle, &proof),
        consent::approve(&db, &clients, &handle, &proof)
    );
    assert_ne!(a.is_ok(), b.is_ok());
    assert!(store::load_request(&db, &clients, &handle).await.is_err());
}

#[tokio::test]
async fn fresh_login_rotates_csrf_and_rejects_stale_or_swapped_sessions() {
    let db = database().await;
    let clients = clients();
    for mode in ["login", "zero_age", "expired_age"] {
        let (old_id, old_session) = session(&db).await;
        sqlx::query("UPDATE identity_sessions SET authenticated_at=clock_timestamp()-interval '10 minutes' WHERE id=$1")
            .bind(old_id).execute(&db).await.unwrap();
        let mut value = input();
        match mode {
            "login" => value.prompt = Some("login".into()),
            "zero_age" => value.max_age = Some(0),
            _ => value.max_age = Some(60),
        }
        let handle = store::create_request(
            &db,
            &value.validate(&clients).unwrap(),
            RequestKind::Authorization,
        )
        .await
        .unwrap();
        let browser = random_secret();
        let started = begin(&db, &clients, &handle, &browser, Some(&old_session))
            .await
            .unwrap();
        assert!(started.needs_login);
        let proof = browser_proof(&browser, &started.csrf_token, Some(&old_session));
        assert!(
            consent::approve(&db, &clients, &handle, &proof)
                .await
                .is_err()
        );
        assert!(
            attach_authenticated_session(&db, &clients, &handle, &proof, &old_session)
                .await
                .is_err()
        );
        let (_, fresh_session) = session(&db).await;
        let csrf = attach_authenticated_session(&db, &clients, &handle, &proof, &fresh_session)
            .await
            .unwrap();
        assert_ne!(csrf, started.csrf_token);
        assert!(
            consent::approve(
                &db,
                &clients,
                &handle,
                &browser_proof(&browser, &started.csrf_token, Some(&fresh_session))
            )
            .await
            .is_err()
        );
        assert!(
            consent::approve(
                &db,
                &clients,
                &handle,
                &browser_proof(&browser, &csrf, Some(&old_session))
            )
            .await
            .is_err()
        );
        assert!(
            consent::approve(
                &db,
                &clients,
                &handle,
                &browser_proof(&browser, &csrf, Some(&fresh_session))
            )
            .await
            .is_ok()
        );
    }
}

#[tokio::test]
async fn denial_consumes_anonymous_interaction_without_creating_a_consent() {
    let db = database().await;
    let clients = clients();
    let handle = request(&db, &clients, RequestKind::Authorization).await;
    let browser = random_secret();
    let started = begin(&db, &clients, &handle, &browser, None).await.unwrap();
    assert!(started.needs_login);
    let proof = browser_proof(&browser, &started.csrf_token, None);
    let response = consent::deny(&db, &clients, &handle, &proof).await.unwrap();
    assert_eq!(response.client_id(), "account-web");
    assert!(consent::deny(&db, &clients, &handle, &proof).await.is_err());
    assert!(store::load_request(&db, &clients, &handle).await.is_err());
}

#[tokio::test]
async fn silent_authorization_requires_a_current_session_and_exact_unexpired_consent() {
    let db = database().await;
    let clients = clients();
    let (session_id, token) = session(&db).await;
    let browser = random_secret();
    let silent_request = |scope: &str| {
        let mut value = input();
        value.prompt = Some("none".into());
        value.scope = scope.into();
        value.validate(&clients).unwrap()
    };
    let request_none = silent_request("openid account:read");
    let first = store::create_request(&db, &request_none, RequestKind::Authorization)
        .await
        .unwrap();
    let start = begin(&db, &clients, &first, &browser, Some(&token))
        .await
        .unwrap();
    assert!(matches!(
        consent::silent(&db, &clients, &first, &browser, Some(&token)).await,
        Err(StoreError::Protocol(OAuthError::ConsentRequired))
    ));
    assert!(
        consent::approve(
            &db,
            &clients,
            &first,
            &browser_proof(&browser, &start.csrf_token, Some(&token))
        )
        .await
        .is_err()
    );

    let handle = request(&db, &clients, RequestKind::Authorization).await;
    let start = begin(&db, &clients, &handle, &browser, Some(&token))
        .await
        .unwrap();
    consent::approve(
        &db,
        &clients,
        &handle,
        &browser_proof(&browser, &start.csrf_token, Some(&token)),
    )
    .await
    .unwrap();
    assert!(
        consent::silent(&db, &clients, &first, &browser, Some(&token))
            .await
            .is_ok()
    );

    for scope in [
        "openid profile account:read",
        "openid offline_access account:read",
    ] {
        let handle = store::create_request(&db, &silent_request(scope), RequestKind::Authorization)
            .await
            .unwrap();
        begin(&db, &clients, &handle, &browser, Some(&token))
            .await
            .unwrap();
        assert!(matches!(
            consent::silent(&db, &clients, &handle, &browser, Some(&token)).await,
            Err(StoreError::Protocol(OAuthError::ConsentRequired))
        ));
    }
    sqlx::query("UPDATE identity_oauth_consents SET granted_at=clock_timestamp()-interval '2 hours',expires_at=clock_timestamp()-interval '1 hour' WHERE principal_id=(SELECT principal_id FROM identity_sessions WHERE id=$1)")
        .bind(session_id).execute(&db).await.unwrap();
    let handle = store::create_request(&db, &request_none, RequestKind::Authorization)
        .await
        .unwrap();
    begin(&db, &clients, &handle, &browser, Some(&token))
        .await
        .unwrap();
    assert!(matches!(
        consent::silent(&db, &clients, &handle, &browser, Some(&token)).await,
        Err(StoreError::Protocol(OAuthError::ConsentRequired))
    ));
    sqlx::query("UPDATE identity_sessions SET revoked_at=clock_timestamp() WHERE id=$1")
        .bind(session_id)
        .execute(&db)
        .await
        .unwrap();
    assert!(matches!(
        consent::silent(&db, &clients, &handle, &browser, Some(&token)).await,
        Err(StoreError::Protocol(OAuthError::LoginRequired))
    ));
}

#[tokio::test]
async fn revoked_session_and_changed_client_registration_cannot_reuse_a_consent_page() {
    let db = database().await;
    let clients = clients();
    let handle = request(&db, &clients, RequestKind::Authorization).await;
    let (id, token) = session(&db).await;
    let browser = random_secret();
    let start = begin(&db, &clients, &handle, &browser, Some(&token))
        .await
        .unwrap();
    let proof = browser_proof(&browser, &start.csrf_token, Some(&token));
    let mut registration = serde_json::to_value(clients.get("account-web").unwrap()).unwrap();
    registration["redirect_uris"] = serde_json::json!(["https://account.example/new-callback"]);
    let changed =
        ClientRegistry::from_json(&serde_json::json!([registration]).to_string(), false).unwrap();
    assert!(
        consent::approve(&db, &changed, &handle, &proof)
            .await
            .is_err()
    );
    sqlx::query("UPDATE identity_sessions SET revoked_at=clock_timestamp() WHERE id=$1")
        .bind(id)
        .execute(&db)
        .await
        .unwrap();
    assert!(
        consent::approve(&db, &clients, &handle, &proof)
            .await
            .is_err()
    );
    assert!(store::load_request(&db, &clients, &handle).await.is_ok());
}
