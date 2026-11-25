use axum::{body::Body, http::Request};
use sqlx::PgPool;
use tower::ServiceExt;

use crate::{
    database::database_test_support::{
        TokenEnvGuard, identity_state, seed_confidential_oauth_client, seed_principal_and_session,
        seed_public_oauth_client,
    },
    oauth::router,
    oauth_clients::{CreateAuthorizationCodeParams, create_authorization_code},
};

#[sqlx::test(migrations = "./migrations")]
async fn authorization_code_grant_issues_access_and_refresh_tokens(pool: PgPool) {
    let _env = TokenEnvGuard::install_test_keys();
    let state = identity_state(pool);
    let client_id = "db-test-oauth-client";
    let client_secret = "db-test-client-secret-value-12345";
    let redirect_uri = "https://app.example.com/oauth/callback";
    seed_confidential_oauth_client(&state.db, client_id, client_secret, redirect_uri).await;
    let (principal_id, session_id, _) = seed_principal_and_session(&state.db).await;
    let code = create_authorization_code(
        &state.db,
        CreateAuthorizationCodeParams {
            client_id,
            principal_id,
            session_id,
            redirect_uri: redirect_uri.to_string(),
            scope: "account:read account:write".to_string(),
            code_challenge: None,
            code_challenge_method: None,
        },
    )
    .await
    .expect("authorization code");

    let app = router(&state);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/oauth/token")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(format!(
                    "grant_type=authorization_code&code={code}&client_id={client_id}&client_secret={client_secret}&redirect_uri={redirect_uri}"
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), axum::http::StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["token_type"], "Bearer");
    assert!(json["access_token"].as_str().is_some_and(|v| !v.is_empty()));
    assert!(
        json["refresh_token"]
            .as_str()
            .is_some_and(|v| !v.is_empty())
    );
    assert_eq!(json["scope"], "account:read account:write");
}

#[sqlx::test(migrations = "./migrations")]
async fn refresh_token_grant_rotates_and_reissues_access_token(pool: PgPool) {
    let _env = TokenEnvGuard::install_test_keys();
    let state = identity_state(pool);
    let client_id = format!("db-test-refresh-{}", uuid::Uuid::new_v4().simple());
    let client_secret = "db-test-refresh-secret-value-123";
    let redirect_uri = "https://app.example.com/oauth/callback";
    seed_confidential_oauth_client(&state.db, &client_id, client_secret, redirect_uri).await;
    let (principal_id, session_id, _) = seed_principal_and_session(&state.db).await;
    let code = create_authorization_code(
        &state.db,
        CreateAuthorizationCodeParams {
            client_id: &client_id,
            principal_id,
            session_id,
            redirect_uri: redirect_uri.to_string(),
            scope: "account:read".to_string(),
            code_challenge: None,
            code_challenge_method: None,
        },
    )
    .await
    .expect("authorization code");

    let app = router(&state);
    let initial = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/oauth/token")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(format!(
                    "grant_type=authorization_code&code={code}&client_id={client_id}&client_secret={client_secret}&redirect_uri={redirect_uri}"
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    let initial_status = initial.status();
    let initial_body = axum::body::to_bytes(initial.into_body(), usize::MAX)
        .await
        .unwrap();
    assert!(
        initial_status.is_success(),
        "authorization_code grant failed: {initial_status} {}",
        String::from_utf8_lossy(&initial_body)
    );
    let initial_json: serde_json::Value = serde_json::from_slice(&initial_body).unwrap();
    let refresh = initial_json["refresh_token"]
        .as_str()
        .expect("refresh token");

    let rotated = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/oauth/token")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(format!(
                    "grant_type=refresh_token&refresh_token={refresh}&client_id={client_id}"
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(rotated.status(), axum::http::StatusCode::OK);
    let rotated_body = axum::body::to_bytes(rotated.into_body(), usize::MAX)
        .await
        .unwrap();
    let rotated_json: serde_json::Value = serde_json::from_slice(&rotated_body).unwrap();
    assert_ne!(rotated_json["refresh_token"], initial_json["refresh_token"]);
    assert!(
        rotated_json["access_token"]
            .as_str()
            .is_some_and(|v| !v.is_empty())
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn authorization_code_grant_rejects_invalid_client_secret(pool: PgPool) {
    let _env = TokenEnvGuard::install_test_keys();
    let state = identity_state(pool);
    let client_id = "db-test-invalid-secret-client";
    let redirect_uri = "https://app.example.com/oauth/callback";
    seed_confidential_oauth_client(&state.db, client_id, "correct-secret", redirect_uri).await;
    let (principal_id, session_id, _) = seed_principal_and_session(&state.db).await;
    let code = create_authorization_code(
        &state.db,
        CreateAuthorizationCodeParams {
            client_id,
            principal_id,
            session_id,
            redirect_uri: redirect_uri.to_string(),
            scope: "account:read".to_string(),
            code_challenge: None,
            code_challenge_method: None,
        },
    )
    .await
    .unwrap();

    let response = router(&state)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/oauth/token")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(format!(
                    "grant_type=authorization_code&code={code}&client_id={client_id}&client_secret=wrong&redirect_uri={redirect_uri}"
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "./migrations")]
async fn authorization_code_requires_client_secret_for_confidential_clients(pool: PgPool) {
    let _env = TokenEnvGuard::install_test_keys();
    let state = identity_state(pool);
    let client_id = "db-test-missing-secret-client";
    let redirect_uri = "https://app.example.com/oauth/callback";
    seed_confidential_oauth_client(&state.db, client_id, "secret-value", redirect_uri).await;
    let (principal_id, session_id, _) = seed_principal_and_session(&state.db).await;
    let code = create_authorization_code(
        &state.db,
        CreateAuthorizationCodeParams {
            client_id,
            principal_id,
            session_id,
            redirect_uri: redirect_uri.to_string(),
            scope: "account:read".to_string(),
            code_challenge: None,
            code_challenge_method: None,
        },
    )
    .await
    .unwrap();

    let response = router(&state)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/oauth/token")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(format!(
                    "grant_type=authorization_code&code={code}&client_id={client_id}&redirect_uri={redirect_uri}"
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "./migrations")]
async fn authorization_code_rejects_unknown_client(pool: PgPool) {
    let _env = TokenEnvGuard::install_test_keys();
    let state = identity_state(pool);
    let response = router(&state)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/oauth/token")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(
                    "grant_type=authorization_code&code=abc&client_id=missing-client&client_secret=s&redirect_uri=https://app.example.com/cb",
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "./migrations")]
async fn public_client_authorization_code_uses_default_audience_without_account_scope(
    pool: PgPool,
) {
    let _env = TokenEnvGuard::install_test_keys();
    let state = identity_state(pool);
    let client_id = format!("db-test-public-{}", uuid::Uuid::new_v4().simple());
    let redirect_uri = "https://app.example.com/oauth/callback";
    seed_public_oauth_client(&state.db, &client_id, redirect_uri).await;
    let (principal_id, session_id, _) = seed_principal_and_session(&state.db).await;
    let code = create_authorization_code(
        &state.db,
        CreateAuthorizationCodeParams {
            client_id: &client_id,
            principal_id,
            session_id,
            redirect_uri: redirect_uri.to_string(),
            scope: "openid profile".to_string(),
            code_challenge: None,
            code_challenge_method: None,
        },
    )
    .await
    .expect("authorization code");

    let response = router(&state)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/oauth/token")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(format!(
                    "grant_type=authorization_code&code={code}&client_id={client_id}&redirect_uri={redirect_uri}"
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), axum::http::StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["access_token"].as_str().is_some_and(|v| !v.is_empty()));
    assert_eq!(json["scope"], "openid profile");
}

#[sqlx::test(migrations = "./migrations")]
async fn authorization_code_rejects_invalid_grant(pool: PgPool) {
    let _env = TokenEnvGuard::install_test_keys();
    let state = identity_state(pool);
    let client_id = "db-test-invalid-grant-client";
    let client_secret = "db-test-invalid-grant-secret";
    let redirect_uri = "https://app.example.com/oauth/callback";
    seed_confidential_oauth_client(&state.db, client_id, client_secret, redirect_uri).await;

    let response = router(&state)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/oauth/token")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(format!(
                    "grant_type=authorization_code&code=not-a-real-code&client_id={client_id}&client_secret={client_secret}&redirect_uri={redirect_uri}"
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
}
