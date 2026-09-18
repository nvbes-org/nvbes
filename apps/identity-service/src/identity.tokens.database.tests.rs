use super::{TokenService, tests::config};
use crate::{
    oauth::{codes, store::RequestKind},
    test_fixtures::{authorize, clients, database, exchange, request, session},
};

#[tokio::test]
async fn replay_revocation_reaches_previously_issued_access_tokens() {
    let db = database().await;
    let clients = clients();
    let service = TokenService::new(config()).unwrap();
    let handle = request(&db, &clients, RequestKind::Authorization).await;
    let (_, session) = session(&db).await;
    let code = authorize(&db, &clients, &handle, &session).await.unwrap();
    let grant = codes::exchange(&db, &clients, exchange(&code.code))
        .await
        .unwrap();
    let response = service.issue_grant(&db, &clients, &grant).await.unwrap();
    assert!(
        service
            .introspect(
                &db,
                &clients,
                &response.access_token,
                "nvbes-account-service"
            )
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        codes::exchange(&db, &clients, exchange(&code.code))
            .await
            .is_err()
    );
    assert!(
        service
            .introspect(
                &db,
                &clients,
                &response.access_token,
                "nvbes-account-service"
            )
            .await
            .unwrap()
            .is_none()
    );
    assert!(service.issue_grant(&db, &clients, &grant).await.is_err());
}

#[tokio::test]
async fn token_response_is_issued_once_even_with_concurrent_callers() {
    let db = database().await;
    let clients = clients();
    let service = TokenService::new(config()).unwrap();
    let handle = request(&db, &clients, RequestKind::Authorization).await;
    let (_, session) = session(&db).await;
    let code = authorize(&db, &clients, &handle, &session).await.unwrap();
    let grant = codes::exchange(&db, &clients, exchange(&code.code))
        .await
        .unwrap();
    let (a, b) = tokio::join!(
        service.issue_grant(&db, &clients, &grant),
        service.issue_grant(&db, &clients, &grant)
    );
    assert_ne!(a.is_ok(), b.is_ok());
}

#[tokio::test]
async fn authorization_snapshot_prevents_later_session_changes_from_elevating_a_code() {
    let db = database().await;
    let clients = clients();
    let service = TokenService::new(config()).unwrap();
    let handle = request(&db, &clients, RequestKind::Authorization).await;
    let (session_id, session) = session(&db).await;
    let code = authorize(&db, &clients, &handle, &session).await.unwrap();
    sqlx::query("UPDATE identity_sessions SET step_up_method='webauthn',step_up_at=clock_timestamp(),step_up_expires_at=clock_timestamp()+interval '5 minutes' WHERE id=$1")
        .bind(session_id).execute(&db).await.unwrap();
    let grant = codes::exchange(&db, &clients, exchange(&code.code))
        .await
        .unwrap();
    let response = service.issue_grant(&db, &clients, &grant).await.unwrap();
    let claims = service
        .verify(&response.access_token, "nvbes-account-service")
        .unwrap();
    assert_eq!(claims.amr, ["pwd"]);
    assert!(claims.step_up_time.is_none());
    sqlx::query("UPDATE identity_sessions SET revoked_at=clock_timestamp() WHERE id=$1")
        .bind(session_id)
        .execute(&db)
        .await
        .unwrap();
    assert!(
        service
            .introspect(
                &db,
                &clients,
                &response.access_token,
                "nvbes-account-service"
            )
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn revocation_before_signing_and_registration_removal_refuse_tokens() {
    let db = database().await;
    let clients = clients();
    let service = TokenService::new(config()).unwrap();
    let handle = request(&db, &clients, RequestKind::Authorization).await;
    let (session_id, session) = session(&db).await;
    let code = authorize(&db, &clients, &handle, &session).await.unwrap();
    let grant = codes::exchange(&db, &clients, exchange(&code.code))
        .await
        .unwrap();
    let mut other = serde_json::to_value(clients.get("account-web").unwrap()).unwrap();
    other["client_id"] = serde_json::json!("another-client");
    let removed = crate::oauth::clients::ClientRegistry::from_json(
        &serde_json::json!([other]).to_string(),
        false,
    )
    .unwrap();
    assert!(service.issue_grant(&db, &removed, &grant).await.is_err());
    sqlx::query("UPDATE identity_principals SET status='suspended' WHERE id=(SELECT principal_id FROM identity_sessions WHERE id=$1)")
        .bind(session_id).execute(&db).await.unwrap();
    assert!(service.issue_grant(&db, &clients, &grant).await.is_err());
}

#[tokio::test]
async fn refresh_rotation_rejects_replay_and_revokes_the_family() {
    let db = database().await;
    let clients = clients();
    let service = TokenService::new(config()).unwrap();
    let mut input = crate::test_fixtures::input();
    input.scope.push_str(" offline_access");
    let validated = input.validate(&clients).unwrap();
    let handle = crate::oauth::store::create_request(&db, &validated, RequestKind::Authorization)
        .await
        .unwrap();
    let (_, session) = session(&db).await;
    let code = authorize(&db, &clients, &handle, &session).await.unwrap();
    let grant = codes::exchange(&db, &clients, exchange(&code.code))
        .await
        .unwrap();
    let first = service.issue_grant(&db, &clients, &grant).await.unwrap();
    let first_refresh = first.refresh_token.clone().unwrap();
    let second = service
        .refresh(
            &db,
            &clients,
            super::RefreshRequest {
                refresh_token: &first_refresh,
                client_id: "account-web",
                dpop_proof: None,
            },
        )
        .await
        .unwrap();
    let second_refresh = second.refresh_token.clone().unwrap();
    assert_ne!(first_refresh, second_refresh);
    assert!(
        service
            .refresh(
                &db,
                &clients,
                super::RefreshRequest {
                    refresh_token: &first_refresh,
                    client_id: "account-web",
                    dpop_proof: None
                }
            )
            .await
            .is_err()
    );
    assert!(
        service
            .refresh(
                &db,
                &clients,
                super::RefreshRequest {
                    refresh_token: &second_refresh,
                    client_id: "account-web",
                    dpop_proof: None
                }
            )
            .await
            .is_err()
    );
    assert!(
        service
            .introspect(&db, &clients, &first.access_token, "nvbes-account-service")
            .await
            .unwrap()
            .is_none()
    );
}
