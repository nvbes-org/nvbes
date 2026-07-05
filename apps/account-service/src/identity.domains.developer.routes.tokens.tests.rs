use axum::{Extension, Json, extract::State};
use chrono::{Duration, Utc};
use uuid::Uuid;

use super::{debug_token, token_access_decision};
use crate::{
    app::{AppConfig, AppState},
    domains::{
        auth::jwt::TokenClaims,
        developer::types::{DebugDeveloperTokenInput, DebugDeveloperTokenResponse},
    },
    http::middleware::jwt::AuthContext,
};

#[tokio::test]
async fn debug_token_reports_decisions_and_records_audit() {
    let pool = crate::test_support::shared_test_pool();
    crate::test_support::ensure_test_database(&pool).await;
    crate::test_support::ensure_test_redis().await;

    let tenant_id = Uuid::new_v4();
    let actor_id = Uuid::new_v4();
    let state = test_state(&pool).await;
    seed_developer_admin(&pool, tenant_id, actor_id).await;
    let auth = test_auth(tenant_id, actor_id);

    let allowed = inspect(
        &state,
        &auth,
        active_token(&state, tenant_id, actor_id).await,
    )
    .await;
    assert!(allowed.active);
    assert_eq!(allowed.access_decision, "allowed");
    assert_eq!(
        allowed
            .claims
            .expect("allowed token should decode")
            .tenant_id,
        Some(tenant_id.to_string())
    );

    let expired = inspect(&state, &auth, expired_token(&state, tenant_id, actor_id)).await;
    assert!(!expired.active);
    assert_eq!(expired.access_decision, "expired");

    let mismatch = inspect(
        &state,
        &auth,
        active_token(&state, Uuid::new_v4(), actor_id).await,
    )
    .await;
    assert!(mismatch.active);
    assert_eq!(mismatch.access_decision, "tenant_mismatch");

    let invalid = inspect(&state, &auth, "not-a-jwt".to_string()).await;
    assert!(!invalid.active);
    assert_eq!(invalid.access_decision, "invalid");
    assert!(invalid.claims.is_none());

    let audit_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM developer_token_debug_sessions WHERE tenant_id = $1 AND actor_principal_id = $2",
    )
    .bind(tenant_id)
    .bind(actor_id)
    .fetch_one(&pool)
    .await
    .expect("audit count should be readable");
    assert_eq!(audit_count, 4);
}

#[test]
fn token_access_decision_allows_active_token_for_tenant() {
    let tenant_id = Uuid::new_v4();
    let tenant = tenant_id.to_string();
    let decision = token_access_decision(100, Some(&tenant), tenant_id, 50, 150);

    assert!(decision.active);
    assert_eq!(decision.access_decision, "allowed");
}

#[test]
fn token_access_decision_marks_expired_token() {
    let tenant_id = Uuid::new_v4();
    let tenant = tenant_id.to_string();
    let decision = token_access_decision(200, Some(&tenant), tenant_id, 50, 150);

    assert!(!decision.active);
    assert_eq!(decision.access_decision, "expired");
}

#[test]
fn token_access_decision_prioritizes_tenant_mismatch() {
    let tenant_id = Uuid::new_v4();
    let other_tenant_id = Uuid::new_v4().to_string();
    let decision = token_access_decision(100, Some(&other_tenant_id), tenant_id, 50, 150);

    assert!(decision.active);
    assert_eq!(decision.access_decision, "tenant_mismatch");
}

async fn inspect(
    state: &AppState,
    auth: &AuthContext,
    access_token: String,
) -> DebugDeveloperTokenResponse {
    debug_token(
        State(state.clone()),
        Extension(auth.clone()),
        Json(DebugDeveloperTokenInput { access_token }),
    )
    .await
    .expect("debug token should succeed")
    .0
}

async fn active_token(state: &AppState, tenant_id: Uuid, subject_id: Uuid) -> String {
    state
        .jwt
        .generate_token_pair_with_session(
            subject_id,
            None,
            None,
            "openid profile drive.files.read",
            Some(Uuid::new_v4()),
            Some(tenant_id),
            None,
            Some("aal1"),
            Some(vec!["pwd".to_string()]),
            Some("developer-console-test"),
            Some(Utc::now().timestamp()),
            None,
        )
        .expect("token pair should be issued")
        .access_token
}

fn expired_token(state: &AppState, tenant_id: Uuid, subject_id: Uuid) -> String {
    let now = Utc::now();
    let claims = TokenClaims {
        jti: Uuid::new_v4().to_string(),
        sid: Uuid::new_v4().to_string(),
        sub: subject_id.to_string(),
        workspace_id: None,
        workspace_region: None,
        tenant_id: Some(tenant_id.to_string()),
        organization_id: None,
        token_type: "access".to_string(),
        scope: "openid profile".to_string(),
        authorization_details: Vec::new(),
        acr: Some("aal1".to_string()),
        amr: vec!["pwd".to_string()],
        client_id: Some("developer-console-test".to_string()),
        auth_time: Some((now - Duration::minutes(30)).timestamp()),
        iss: "nvbes-identity".to_string(),
        aud: "nvbes-account-service".to_string(),
        exp: (now - Duration::minutes(1)).timestamp(),
        iat: (now - Duration::minutes(30)).timestamp(),
        nbf: (now - Duration::minutes(30)).timestamp(),
        cnf: None,
        act: None,
    };

    state
        .jwt
        .encode_token(&claims)
        .expect("expired token should encode")
}

async fn seed_developer_admin(pool: &sqlx::PgPool, tenant_id: Uuid, principal_id: Uuid) {
    let now = Utc::now();
    sqlx::query(
        "INSERT INTO tenants (id, kind, name, slug, status, security_tier, created_at, updated_at)
         VALUES ($1, 'team', 'Token Debug Test Tenant', $2, 'active', 'standard', $3, $3)",
    )
    .bind(tenant_id)
    .bind(format!("token-debug-test-{tenant_id}"))
    .bind(now)
    .execute(pool)
    .await
    .expect("tenant should be seeded");

    sqlx::query(
        "INSERT INTO principals (id, tenant_id, principal_kind, status, display_name, created_at, updated_at)
         VALUES ($1, $2, 'human', 'active', 'Token Debugger', $3, $3)",
    )
    .bind(principal_id)
    .bind(tenant_id)
    .bind(now)
    .execute(pool)
    .await
    .expect("principal should be seeded");

    sqlx::query(
        "INSERT INTO tenant_memberships (tenant_id, principal_id, principal_kind, role, status, created_at, updated_at)
         VALUES ($1, $2, 'human', 'member', 'active', $3, $3)",
    )
    .bind(tenant_id)
    .bind(principal_id)
    .bind(now)
    .execute(pool)
    .await
    .expect("tenant membership should be seeded");

    sqlx::query(
        "INSERT INTO developer_role_assignments (tenant_id, principal_id, role, created_at)
         VALUES ($1, $2, 'developer_admin', $3)",
    )
    .bind(tenant_id)
    .bind(principal_id)
    .bind(now)
    .execute(pool)
    .await
    .expect("developer role should be seeded");
}

fn test_auth(tenant_id: Uuid, principal_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: principal_id,
        user_email: "token-debugger@example.com".to_string(),
        display_name: "Token Debugger".to_string(),
        email_verified_at: Some(Utc::now()),
        mfa_enabled: false,
        tenant_id: Some(tenant_id),
        organization_id: None,
        workspace_id: None,
        workspace_region: None,
        token_type: "Bearer".to_string(),
        scope: String::new(),
        jti: "token-debugger-test".to_string(),
        session_id: Uuid::new_v4(),
        acr: None,
        amr: Vec::new(),
        auth_time: None,
        client_id: None,
        cnf_jkt: None,
    }
}

async fn test_state(pool: &sqlx::PgPool) -> AppState {
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

    let config = AppConfig {
        database_url: std::env::var("DATABASE_URL")
            .or_else(|_| std::env::var("NVBES_DATABASE_URL"))
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/nvbes".to_string()),
        environment: "development".to_string(),
        app_name: "identity-token-debugger-test".to_string(),
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

    AppState::bootstrap(&config, pool.clone())
        .await
        .expect("app state bootstrap should succeed")
}
