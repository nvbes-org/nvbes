use axum::{body::Body, http::Request};
use tower::ServiceExt;

use crate::authz::router;
use crate::database::database_test_support::WithoutTokenKeysGuard;
use crate::health::tests::state;

#[tokio::test]
async fn introspect_fails_cleanly_without_configured_token_keys() {
    let _env = WithoutTokenKeysGuard::install();
    let app = router(&state());
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/oauth/introspect")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"token":"invalid.token.here"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), axum::http::StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn introspect_rejects_malformed_json_body() {
    let app = router(&state());
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/oauth/introspect")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"token":}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), axum::http::StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn authz_decision_fails_cleanly_without_configured_token_keys() {
    let _env = WithoutTokenKeysGuard::install();
    let app = router(&state());
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/authz/decision")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"token":"invalid.token","resource":"user","action":"read"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), axum::http::StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn authz_decision_rejects_malformed_json_body() {
    let app = router(&state());
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/authz/decision")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"token":}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), axum::http::StatusCode::BAD_REQUEST);
}

#[cfg(feature = "database-tests")]
mod database {
    use axum::{body::Body, http::Request};
    use sqlx::PgPool;
    use tower::ServiceExt;
    use uuid::Uuid;

    use crate::authz::router;
    use crate::database::database_test_support::{
        TokenEnvGuard, identity_state, seed_principal_and_session,
    };
    use crate::tokens::TokenService;
    use crate::tokens_config::TokenConfig;

    #[sqlx::test(migrations = "./migrations")]
    async fn introspect_reports_active_account_token(pool: PgPool) {
        let _env = TokenEnvGuard::install_test_keys();
        let token_config = TokenConfig::from_env("test").expect("token config");
        let service = TokenService::new(token_config).expect("token service");
        let (principal_id, session_id, _) = seed_principal_and_session(&pool).await;
        let access_token = service
            .issue(
                principal_id,
                session_id,
                "account",
                "account:read account:write",
                vec!["pwd".to_string()],
            )
            .expect("access token");

        let state = identity_state(pool);
        let app = router(&state);
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/oauth/introspect")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"token":"{}","token_type_hint":"account"}}"#,
                        access_token
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
        assert_eq!(json["active"], true);
        assert!(json["scope"].as_str().is_some_and(|s| !s.is_empty()));
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn introspect_reports_inactive_for_unknown_token(pool: PgPool) {
        let _env = TokenEnvGuard::install_test_keys();
        let state = identity_state(pool);
        let response = router(&state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/oauth/introspect")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"token":"not.a.valid.jwt","token_type_hint":"account"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["active"], false);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn authz_decision_allows_scoped_token_and_denies_empty_scope(pool: PgPool) {
        let _env = TokenEnvGuard::install_test_keys();
        let token_config = TokenConfig::from_env("test").expect("token config");
        let service = TokenService::new(token_config.clone()).expect("token service");
        let (principal_id, session_id, _) = seed_principal_and_session(&pool).await;
        let access_token = service
            .issue(
                principal_id,
                session_id,
                "account",
                "account:read account:write",
                vec!["pwd".to_string()],
            )
            .expect("access token");
        let empty_scope_token = {
            use crate::tokens::AccessTokenClaims;
            use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
            let issued_at = chrono::Utc::now().timestamp() as u64;
            let claims = AccessTokenClaims {
                sub: principal_id.to_string(),
                token_type: "access".into(),
                scope: String::new(),
                amr: vec!["pwd".to_string()],
                iss: token_config.issuer.clone(),
                aud: "account".into(),
                exp: issued_at + 900,
                iat: issued_at,
                nbf: issued_at,
                jti: Uuid::new_v4().to_string(),
                sid: session_id.to_string(),
            };
            let mut header = Header::new(Algorithm::RS256);
            header.kid = Some(token_config.key_id.clone());
            header.typ = Some("at+jwt".into());
            let encoding_key =
                EncodingKey::from_rsa_pem(token_config.private_key_pem.as_bytes()).expect("key");
            encode(&header, &claims, &encoding_key).expect("empty scope token")
        };
        let state = identity_state(pool);
        let app = router(&state);
        let workspace_id = Uuid::new_v4();

        let allowed = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/authz/decision")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"token":"{access_token}","resource":"profile","action":"read","workspace_id":"{workspace_id}"}}"#
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(allowed.status(), axum::http::StatusCode::OK);
        let allowed_body = axum::body::to_bytes(allowed.into_body(), usize::MAX)
            .await
            .unwrap();
        let allowed_json: serde_json::Value = serde_json::from_slice(&allowed_body).unwrap();
        assert_eq!(allowed_json["allowed"], true);
        assert_eq!(
            allowed_json["context"]["principal_id"],
            principal_id.to_string()
        );
        assert_eq!(
            allowed_json["context"]["workspace_id"],
            workspace_id.to_string()
        );

        let empty_scope = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/authz/decision")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"token":"{empty_scope_token}","resource":"profile","action":"read"}}"#
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(empty_scope.status(), axum::http::StatusCode::OK);
        let empty_body = axum::body::to_bytes(empty_scope.into_body(), usize::MAX)
            .await
            .unwrap();
        let empty_json: serde_json::Value = serde_json::from_slice(&empty_body).unwrap();
        assert_eq!(empty_json["allowed"], false);
        assert!(
            empty_json["reason"]
                .as_str()
                .is_some_and(|r| r.contains("no authorized scopes"))
        );

        let denied = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/authz/decision")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"token":"not.a.valid.jwt","resource":"profile","action":"read"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(denied.status(), axum::http::StatusCode::OK);
        let denied_body = axum::body::to_bytes(denied.into_body(), usize::MAX)
            .await
            .unwrap();
        let denied_json: serde_json::Value = serde_json::from_slice(&denied_body).unwrap();
        assert_eq!(denied_json["allowed"], false);
        assert_eq!(denied_json["reason"], "Token is invalid or expired");
    }
}
