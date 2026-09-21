use base64::Engine;
use chrono::{Duration, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header};
use openssl::{pkey::PKey, rsa::Rsa};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    auth::hash_token,
    oauth_clients::{
        CreateAuthorizationCodeParams, create_authorization_code, get_oauth_client,
        is_redirect_uri_allowed, validate_and_consume_authorization_code,
        validate_client_credentials,
    },
    refresh::{create_refresh_token, rotate_refresh_token},
    tokens::TokenService,
    tokens_config::TokenConfig,
};

async fn create_test_principal_and_session(pool: &PgPool) -> (Uuid, Uuid) {
    let principal_id = Uuid::new_v4();
    let session_id = Uuid::new_v4();

    sqlx::query(
        "INSERT INTO identity_principals (id, kind, status) VALUES ($1, 'human', 'active')",
    )
    .bind(principal_id)
    .execute(pool)
    .await
    .expect("principal created");

    sqlx::query(
        "INSERT INTO identity_sessions (id, principal_id, token_hash, expires_at) 
         VALUES ($1, $2, $3, $4)",
    )
    .bind(session_id)
    .bind(principal_id)
    .bind(hash_token("test-session-secret"))
    .bind(Utc::now() + Duration::hours(24))
    .execute(pool)
    .await
    .expect("session created");

    (principal_id, session_id)
}

#[sqlx::test(migrations = "./migrations")]
async fn refresh_token_rotation_and_family_reuse_revocation(pool: PgPool) {
    let (principal_id, session_id) = create_test_principal_and_session(&pool).await;
    let client_id = "test-client-app";
    let scope = "account:read account:write";

    // 1. Initial refresh token creation
    let token_1 = create_refresh_token(&pool, principal_id, session_id, client_id, scope)
        .await
        .expect("create refresh token 1");

    assert_eq!(token_1.principal_id, principal_id);
    assert_eq!(token_1.session_id, session_id);
    assert_eq!(token_1.scope, scope);

    // 2. First rotation: token 1 -> token 2
    let token_2 = rotate_refresh_token(&pool, &token_1.token, client_id)
        .await
        .expect("rotate token 1 to 2");

    assert_ne!(token_1.token, token_2.token);
    assert_eq!(token_2.principal_id, principal_id);

    // 3. Second rotation: token 2 -> token 3
    let token_3 = rotate_refresh_token(&pool, &token_2.token, client_id)
        .await
        .expect("rotate token 2 to 3");

    assert_ne!(token_2.token, token_3.token);

    // 4. Attack simulation: replay token 1 (already revoked)
    let replay_err = rotate_refresh_token(&pool, &token_1.token, client_id)
        .await
        .expect_err("replay of token 1 must be detected");

    assert!(
        replay_err.to_string().contains("reuse detected"),
        "error message should indicate reuse: {}",
        replay_err
    );

    // 5. Verify that token 3 was revoked as part of the family revocation
    let active_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM identity_refresh_tokens 
         WHERE principal_id = $1 AND revoked_at IS NULL",
    )
    .bind(principal_id)
    .fetch_one(&pool)
    .await
    .expect("count active refresh tokens");

    assert_eq!(active_count, 0, "all tokens in family must be revoked");
}

#[sqlx::test(migrations = "./migrations")]
async fn oauth_client_and_pkce_authorization_code_invariants(pool: PgPool) {
    let (principal_id, session_id) = create_test_principal_and_session(&pool).await;
    let client_id = "test-oauth-client";
    let client_secret = "super-secret-client-credential-123456";
    let redirect_uri = "https://app.example.com/oauth/callback";

    // 1. Register OAuth client in database
    sqlx::query(
        "INSERT INTO identity_oauth_clients (id, client_id, client_secret_hash, name, redirect_uris, scopes, is_confidential) 
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(Uuid::new_v4())
    .bind(client_id)
    .bind(hash_token(client_secret))
    .bind("Test App")
    .bind(vec![redirect_uri.to_string()])
    .bind(vec!["account:read".to_string(), "account:write".to_string()])
    .bind(true)
    .execute(&pool)
    .await
    .expect("client inserted");

    // 2. Validate client lookups and credentials
    let client = get_oauth_client(&pool, client_id)
        .await
        .expect("get client")
        .expect("client found");
    assert_eq!(client.client_id, client_id);
    assert!(client.is_confidential);

    assert!(
        validate_client_credentials(&pool, client_id, client_secret)
            .await
            .unwrap()
    );
    assert!(
        !validate_client_credentials(&pool, client_id, "wrong-secret")
            .await
            .unwrap()
    );
    assert!(
        is_redirect_uri_allowed(&pool, client_id, redirect_uri)
            .await
            .unwrap()
    );
    assert!(
        !is_redirect_uri_allowed(&pool, client_id, "https://evil.com/cb")
            .await
            .unwrap()
    );

    // 3. Create Authorization Code with PKCE (S256)
    let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
    let hash = Sha256::digest(verifier.as_bytes());
    let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(hash);

    let code = create_authorization_code(
        &pool,
        CreateAuthorizationCodeParams {
            client_id,
            principal_id,
            session_id,
            redirect_uri: redirect_uri.to_string(),
            scope: "account:read".to_string(),
            code_challenge: Some(challenge),
            code_challenge_method: Some("S256".to_string()),
        },
    )
    .await
    .expect("code created");

    // 4. Attempt consumption with wrong PKCE verifier -> fail
    let bad_pkce = validate_and_consume_authorization_code(
        &pool,
        &code,
        client_id,
        redirect_uri,
        Some("wrong-verifier-value"),
    )
    .await;
    assert!(bad_pkce.is_err(), "wrong verifier must fail");

    // 5. Consume with correct PKCE verifier -> succeed
    let code_info = validate_and_consume_authorization_code(
        &pool,
        &code,
        client_id,
        redirect_uri,
        Some(verifier),
    )
    .await
    .expect("consume with valid verifier");

    assert_eq!(code_info.principal_id, principal_id);
    assert_eq!(code_info.session_id, session_id);
    assert_eq!(code_info.scope, "account:read");

    // 6. One-time consumption: replay must fail immediately
    let replay = validate_and_consume_authorization_code(
        &pool,
        &code,
        client_id,
        redirect_uri,
        Some(verifier),
    )
    .await;
    assert!(replay.is_err(), "already consumed code cannot be reused");
}

#[sqlx::test(migrations = "./migrations")]
async fn identity_account_token_contract_integration(pool: PgPool) {
    let (principal_id, session_id) = create_test_principal_and_session(&pool).await;

    let rsa = Rsa::generate(2048).unwrap();
    let private = PKey::from_rsa(rsa).unwrap();
    let private_pem = String::from_utf8(private.private_key_to_pem_pkcs8().unwrap()).unwrap();
    let public_pem = String::from_utf8(private.public_key_to_pem().unwrap()).unwrap();

    let service = TokenService::new(
        TokenConfig::from_values(
            "test",
            "https://identity.nvbes.eu".into(),
            "identity-key-2026".into(),
            private_pem,
            public_pem.clone(),
            "account,billing".into(),
        )
        .unwrap(),
    )
    .unwrap();

    // 1. Issue access token for audience "account" with step-up MFA
    let token = service
        .issue(
            principal_id,
            session_id,
            "account",
            "account:read account:write account:close",
            vec!["pwd".to_string(), "totp".to_string()],
        )
        .expect("issue token");

    // 2. Validate using exact RS256 decoding parameters expected by Account service
    let header = decode_header(&token).unwrap();
    assert_eq!(header.alg, Algorithm::RS256);
    assert_eq!(header.kid.as_deref(), Some("identity-key-2026"));
    assert_eq!(header.typ.as_deref(), Some("at+jwt"));

    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_issuer(&["https://identity.nvbes.eu"]);
    validation.set_audience(&["account"]);
    validation.set_required_spec_claims(&["exp", "iat", "iss", "aud", "sub", "nbf"]);

    let decoding_key = DecodingKey::from_rsa_pem(public_pem.as_bytes()).unwrap();
    let token_data = decode::<serde_json::Value>(&token, &decoding_key, &validation).unwrap();
    let claims = token_data.claims;

    assert_eq!(claims["token_type"], "access");
    assert_eq!(claims["sub"], principal_id.to_string());
    assert_eq!(claims["sid"], session_id.to_string());
    assert_eq!(claims["aud"], "account");
    assert_eq!(claims["scope"], "account:read account:write account:close");

    let amr = claims["amr"].as_array().unwrap();
    assert!(amr.iter().any(|m| m.as_str() == Some("totp")));
}
