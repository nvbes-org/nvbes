use super::*;
use axum::http::header::SET_COOKIE;
use axum::{
    body,
    extract::{Query, State},
};
use chrono::Utc;
use sqlx::PgPool;

use super::mfa::challenge_mfa;
use super::pwd::challenge_pwd;
use super::types::{MfaRequest, PwdRequest};

fn test_pool() -> PgPool {
    crate::test_support::shared_test_pool()
}

async fn test_config(pool: &PgPool) -> AppState {
    unsafe {
        std::env::set_var("NVBES_ENV", "development");
        std::env::set_var(
            "NVBES_WORKSPACE_ROOT",
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .ancestors()
                .nth(2)
                .expect("workspace root"),
        );
    }

    let config = crate::app::AppConfig {
        database_url: std::env::var("DATABASE_URL")
            .or_else(|_| std::env::var("NVBES_DATABASE_URL"))
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/nvbes".to_string()),
        environment: "development".to_string(),
        app_name: "account-service-test".to_string(),
        email_provider: "mock".to_string(),
        otp_provider: "mock".to_string(),
        redis_url: std::env::var("NVBES_REDIS_URL")
            .unwrap_or_else(|_| "redis://localhost:6379".to_string()),
        redis_password: std::env::var("NVBES_REDIS_PASSWORD")
            .ok()
            .filter(|value| !value.trim().is_empty()),
        redis_max_connections: std::env::var("NVBES_REDIS_MAX_CONNECTIONS")
            .ok()
            .and_then(|value| value.parse::<u32>().ok())
            .unwrap_or(10),
        ..Default::default()
    };

    crate::test_support::ensure_test_redis().await;
    crate::test_support::ensure_test_database(pool).await;
    crate::test_support::cloud_mock::ensure_cloud_mock(pool);

    crate::app::AppState::bootstrap(&config, pool.clone())
        .await
        .expect("app state bootstrap should succeed")
}

async fn seed_login_subject(pool: &PgPool, email: &str, password: &str) -> (Uuid, Uuid) {
    crate::test_support::ensure_test_redis().await;
    crate::test_support::ensure_test_database(pool).await;

    let tenant_id = Uuid::new_v4();
    let principal_id = Uuid::new_v4();
    let now = Utc::now();
    let password_hash =
        crate::domains::auth::hash_password(password).expect("password should hash");

    sqlx::query(
        r#"
        INSERT INTO tenants (id, kind, name, slug, status, security_tier, created_at, updated_at)
        VALUES ($1, 'personal', 'Test Tenant', $2, 'active', 'standard', $3, $3)
        "#,
    )
    .bind(tenant_id)
    .bind(format!("tenant-{}", tenant_id))
    .bind(now)
    .execute(pool)
    .await
    .expect("tenant insert should succeed");

    sqlx::query(
        r#"
        INSERT INTO principals (id, tenant_id, principal_kind, status, display_name, created_at, updated_at)
        VALUES ($1, $2, 'human', 'active', 'Login Tester', $3, $3)
        "#,
    )
    .bind(principal_id)
    .bind(tenant_id)
    .bind(now)
    .execute(pool)
    .await
    .expect("principal insert should succeed");

    sqlx::query(
        r#"
        INSERT INTO users (
          principal_id, email, password_hash,
          email_verified_at, status, created_at, updated_at
        )
        VALUES ($1, $2, $3, NOW(), 'active', $4, $4)
        "#,
    )
    .bind(principal_id)
    .bind(email)
    .bind(password_hash)
    .bind(now)
    .execute(pool)
    .await
    .expect("user insert should succeed");

    (tenant_id, principal_id)
}

async fn insert_recovery_codes(pool: &PgPool, principal_id: Uuid, code: &str) {
    sqlx::query(
        r#"
        INSERT INTO mfa_factors (
          id, principal_id, factor_type, status, label, factor_data, confirmed_at, created_at
        )
        VALUES ($1, $2, 'recovery_code', 'active', 'Recovery codes', $3, NOW(), NOW())
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(principal_id)
    .bind(serde_json::json!({
        "codes": [crate::domains::auth::token_hash(code)]
    }))
    .execute(pool)
    .await
    .expect("recovery code insert should succeed");
}

async fn response_json(response: Response) -> serde_json::Value {
    let bytes = body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    serde_json::from_slice(&bytes).expect("response body should be valid json")
}

fn session_id_from_body(body: &serde_json::Value) -> Uuid {
    Uuid::parse_str(
        body["session"]["id"]
            .as_str()
            .expect("login response should include a session id"),
    )
    .expect("login response session id should be a UUID")
}

async fn cleanup_tenant(pool: &PgPool, tenant_id: Uuid) {
    sqlx::query("DELETE FROM tenants WHERE id = $1")
        .bind(tenant_id)
        .execute(pool)
        .await
        .ok();
}

#[tokio::test]
async fn challenge_pwd_allows_a_normal_first_device_without_an_mfa_factor() {
    let pool = test_pool();
    let state = test_config(&pool).await;
    let email = format!("login-first-device-{}@example.com", Uuid::new_v4());
    let password = "Sup3rS3cret!";
    let (tenant_id, principal_id) = seed_login_subject(&pool, &email, password).await;
    let state_token = create_state(
        &state.redis,
        Some(principal_id),
        &email,
        "pwd",
        None,
        Vec::new(),
    )
    .await
    .expect("auth state should be created");

    let response = challenge_pwd(
        State(state),
        Query(LoginQuery { authuser: None }),
        HeaderMap::new(),
        Json(PwdRequest {
            state_token,
            password: password.to_string(),
        }),
    )
    .await
    .expect("normal first-device authentication should succeed");

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(
        body["user"]["id"],
        serde_json::Value::String(principal_id.to_string())
    );
    cleanup_tenant(&pool, tenant_id).await;
}

#[tokio::test]
async fn challenge_pwd_returns_mfa_step_when_recovery_codes_are_configured() {
    let pool = test_pool();
    let state = test_config(&pool).await;
    let email = format!("login-mfa-{}@example.com", Uuid::new_v4());
    let password = "Sup3rS3cret!";
    let recovery_code = "RECOVERY-1234";
    let (tenant_id, principal_id) = seed_login_subject(&pool, &email, password).await;
    insert_recovery_codes(&pool, principal_id, recovery_code).await;

    let state_token = create_state(
        &state.redis,
        Some(principal_id),
        &email,
        "pwd",
        None,
        Vec::new(),
    )
    .await
    .expect("auth state should be created");

    let response = challenge_pwd(
        State(state),
        Query(LoginQuery { authuser: None }),
        HeaderMap::new(),
        Json(PwdRequest {
            state_token,
            password: password.to_string(),
        }),
    )
    .await
    .expect("password challenge should succeed");

    assert_eq!(response.status(), StatusCode::ACCEPTED);

    let body = response_json(response).await;
    assert_eq!(body["next_step"], "mfa");
    assert!(body["state_token"].as_str().is_some());
    assert_eq!(body["available_methods"], serde_json::json!(["recovery"]));

    cleanup_tenant(&pool, tenant_id).await;
}

#[tokio::test]
async fn challenge_mfa_creates_aal2_session_and_sets_cookie() {
    let pool = test_pool();
    let state = test_config(&pool).await;
    let email = format!("login-mfa-finish-{}@example.com", Uuid::new_v4());
    let password = "Sup3rS3cret!";
    let recovery_code = "RECOVERY-5678";
    let (tenant_id, principal_id) = seed_login_subject(&pool, &email, password).await;
    insert_recovery_codes(&pool, principal_id, recovery_code).await;

    let state_token = create_state(
        &state.redis,
        Some(principal_id),
        &email,
        "mfa",
        None,
        vec!["pwd".to_string()],
    )
    .await
    .expect("mfa auth state should be created");

    let response = challenge_mfa(
        State(state.clone()),
        Query(LoginQuery { authuser: None }),
        HeaderMap::new(),
        Json(MfaRequest {
            state_token,
            totp_code: None,
            recovery_code: Some(recovery_code.to_string()),
            webauthn_response: None,
            webauthn_challenge_id: None,
        }),
    )
    .await
    .expect("mfa challenge should succeed");

    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers().get(SET_COOKIE).is_some());

    let body = response_json(response).await;
    let session_id = session_id_from_body(&body);
    let session = nvbes_redis::session::get_session(&state.redis, &session_id.to_string())
        .await
        .expect("redis session lookup should succeed")
        .expect("session should exist");

    assert_eq!(session.acr.as_deref(), Some("aal2"));
    assert_eq!(session.amr, vec!["pwd".to_string(), "recovery".to_string()]);

    cleanup_tenant(&pool, tenant_id).await;
}

#[tokio::test]
async fn challenge_pwd_preserves_verified_webauthn_and_creates_aal2_session() {
    let pool = test_pool();
    let state = test_config(&pool).await;
    let email = format!("login-passkey-password-{}@example.com", Uuid::new_v4());
    let password = "Sup3rS3cret!";
    let (tenant_id, principal_id) = seed_login_subject(&pool, &email, password).await;

    let state_token = create_state(
        &state.redis,
        Some(principal_id),
        &email,
        "pwd",
        None,
        vec!["webauthn".to_string()],
    )
    .await
    .expect("post-WebAuthn password state should be created");

    let response = challenge_pwd(
        State(state.clone()),
        Query(LoginQuery { authuser: None }),
        HeaderMap::new(),
        Json(PwdRequest {
            state_token,
            password: password.to_string(),
        }),
    )
    .await
    .expect("password challenge should complete the login");

    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers().get(SET_COOKIE).is_some());

    let body = response_json(response).await;
    let session_id = session_id_from_body(&body);
    let session = nvbes_redis::session::get_session(&state.redis, &session_id.to_string())
        .await
        .expect("redis session lookup should succeed")
        .expect("session should exist");

    assert_eq!(session.acr.as_deref(), Some("aal2"));
    assert_eq!(session.amr, vec!["webauthn".to_string(), "pwd".to_string()]);

    cleanup_tenant(&pool, tenant_id).await;
}
