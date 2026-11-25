use crate::{
    oauth::{
        clients::ClientRegistry,
        consent, interactions,
        store::{self, RequestKind},
    },
    test_fixtures::{self, browser_proof, isolated_database, session},
    tokens::{AuthorizationCodeRequest, RefreshRequest, TokenService},
};

fn registry(policy: &str) -> ClientRegistry {
    let mut value =
        serde_json::to_value(test_fixtures::clients().get("account-web").unwrap()).unwrap();
    value["minimum_authentication"] = serde_json::json!(policy);
    ClientRegistry::from_json(&serde_json::json!([value]).to_string(), false).unwrap()
}

fn exchange(code: &str) -> AuthorizationCodeRequest<'_> {
    AuthorizationCodeRequest {
        code,
        client_id: "account-web",
        redirect_uri: "https://account.example/callback",
        verifier: "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk",
        dpop_proof: None,
    }
}

#[tokio::test]
async fn password_cannot_consent_but_real_totp_can_retry_without_losing_interaction() {
    let db = isolated_database().await;
    let clients = registry("recent_mfa");
    let mut input = test_fixtures::input();
    input.scope.push_str(" offline_access");
    let handle = store::create_request(
        &db,
        &input.validate(&clients).unwrap(),
        RequestKind::Authorization,
    )
    .await
    .unwrap();
    let (_, token) = session(&db).await;
    let browser = store::random_secret();
    let started = interactions::begin(&db, &clients, &handle, &browser, Some(&token))
        .await
        .unwrap();
    let proof = browser_proof(&browser, &started.csrf_token, Some(&token));
    assert!(
        consent::approve(&db, &clients, &handle, &proof)
            .await
            .is_err()
    );
    let codes: i64 = sqlx::query_scalar("SELECT count(*) FROM identity_oauth_codes")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(codes, 0);
    let crypto = crate::mfa_crypto::MfaCrypto::with_rotation(1, [8; 32], None).unwrap();
    let pending = crate::totp::start(&db, &crypto, &token).await.unwrap();
    let code = nvbes_core::mfa::generate_totp_code(
        &pending.secret_base32,
        nvbes_core::mfa::current_counter(chrono::Utc::now()),
    );
    crate::totp::confirm(&db, &crypto, &token, pending.factor_id, &code)
        .await
        .unwrap();
    assert!(
        consent::approve(&db, &registry("recent_webauthn"), &handle, &proof)
            .await
            .is_err()
    );
    let authorized = consent::approve(&db, &clients, &handle, &proof)
        .await
        .unwrap();
    let service = TokenService::new(crate::tokens::tests::config()).unwrap();
    let result = service
        .exchange_code(&db, &clients, exchange(&authorized.code))
        .await
        .unwrap();
    let claims = service
        .verify(&result.access_token, "nvbes-account-service")
        .unwrap();
    assert!(claims.amr.iter().any(|amr| amr == "totp"));
    assert!(claims.exp <= claims.step_up_time.unwrap() + 300);
    // Age the immutable grant snapshot, while keeping a coherent proof sequence.
    // A newer session proof must not silently elevate an already issued grant.
    sqlx::query("UPDATE identity_oauth_codes SET authentication=jsonb_set(jsonb_set(authentication,'{authenticated_at}',to_jsonb(clock_timestamp()-interval '20 minutes')),'{step_up_at}',to_jsonb(clock_timestamp()-interval '6 minutes'))")
        .execute(&db).await.unwrap();
    assert!(
        service
            .introspect(&db, &clients, &result.access_token, "nvbes-account-service")
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        service
            .refresh(
                &db,
                &clients,
                RefreshRequest {
                    refresh_token: &result.refresh_token.unwrap(),
                    client_id: "account-web",
                    dpop_proof: None
                }
            )
            .await
            .is_err()
    );
}

#[tokio::test]
async fn registration_tightening_blocks_exchange_refresh_and_live_introspection() {
    let db = isolated_database().await;
    let clients = registry("primary");
    let strong = registry("recent_mfa");
    let mut input = test_fixtures::input();
    input.scope.push_str(" offline_access");
    let handle = store::create_request(
        &db,
        &input.validate(&clients).unwrap(),
        RequestKind::Authorization,
    )
    .await
    .unwrap();
    let (_, token) = session(&db).await;
    let code = test_fixtures::authorize(&db, &clients, &handle, &token)
        .await
        .unwrap();
    let service = TokenService::new(crate::tokens::tests::config()).unwrap();
    assert!(
        service
            .exchange_code(&db, &strong, exchange(&code.code))
            .await
            .is_err()
    );
    // Rejected issuance rolled back code consumption; explicit policy relaxation allows retry.
    let result = service
        .exchange_code(&db, &clients, exchange(&code.code))
        .await
        .unwrap();
    assert!(
        service
            .introspect(&db, &clients, &result.access_token, "nvbes-account-service")
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        service
            .introspect(&db, &strong, &result.access_token, "nvbes-account-service")
            .await
            .unwrap()
            .is_none()
    );
    let refresh = result.refresh_token.unwrap();
    assert!(
        service
            .refresh(
                &db,
                &strong,
                RefreshRequest {
                    refresh_token: &refresh,
                    client_id: "account-web",
                    dpop_proof: None
                }
            )
            .await
            .is_err()
    );
    let rotated: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM identity_oauth_refresh_tokens WHERE rotated_at IS NOT NULL",
    )
    .fetch_one(&db)
    .await
    .unwrap();
    assert_eq!(rotated, 0);
}

#[tokio::test]
async fn silent_consent_cannot_bypass_stronger_current_registration() {
    let db = isolated_database().await;
    let clients = registry("primary");
    let handle = test_fixtures::request(&db, &clients, RequestKind::Authorization).await;
    let (_, token) = session(&db).await;
    test_fixtures::authorize(&db, &clients, &handle, &token)
        .await
        .unwrap();
    let mut input = test_fixtures::input();
    input.prompt = Some("none".into());
    let handle = store::create_request(
        &db,
        &input.validate(&clients).unwrap(),
        RequestKind::Authorization,
    )
    .await
    .unwrap();
    let browser = store::random_secret();
    interactions::begin(&db, &clients, &handle, &browser, Some(&token))
        .await
        .unwrap();
    assert!(
        consent::silent(
            &db,
            &registry("recent_mfa"),
            &handle,
            &browser,
            Some(&token)
        )
        .await
        .is_err()
    );
    assert!(
        consent::silent(&db, &clients, &handle, &browser, Some(&token))
            .await
            .is_ok()
    );
}
