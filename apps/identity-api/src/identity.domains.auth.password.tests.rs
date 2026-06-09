use super::{hash_password, reset, token_hash, verify_password};
use crate::domains::auth::sessions::cache::cached_session_from_login;
use crate::domains::auth::sessions::cache::current_session_ttl;
use crate::domains::auth::types::ResetPasswordInput;
use chrono::Utc;
use nvbes_core::config::AppConfig;
use nvbes_redis::password_reset::{self, CachedPasswordResetToken};
use nvbes_redis::refresh_token::{self, CachedRefreshToken};
use sqlx::{PgPool, Row};
use uuid::Uuid;

fn test_pool() -> PgPool {
    crate::test_support::shared_test_pool()
}

async fn test_redis_pool() -> nvbes_redis::RedisPool {
    crate::test_support::test_redis_pool().await
}

async fn seed_subject(pool: &PgPool, redis: &nvbes_redis::RedisPool) -> (Uuid, Uuid, Uuid, String) {
    crate::test_support::ensure_test_database(pool).await;

    let tenant_id = Uuid::new_v4();
    let principal_id = Uuid::new_v4();
    let session_id = Uuid::new_v4();
    let reset_token = format!("reset-{}", Uuid::new_v4());
    let email = format!("reset-password-{}@example.com", Uuid::new_v4());
    let now = Utc::now();
    let password_hash = hash_password("Old$trongPassw0rd!").expect("password should hash");

    sqlx::query(
        r#"
        INSERT INTO tenants (id, kind, name, slug, status, security_tier, created_at, updated_at)
        VALUES ($1, 'personal', 'Reset Tenant', $2, 'active', 'standard', $3, $3)
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
        VALUES ($1, $2, 'human', 'active', 'Reset Tester', $3, $3)
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
          principal_id, email, firstname, lastname, username, password_hash,
          email_verified_at, status, created_at, updated_at
        )
        VALUES ($1, $2, 'Reset', 'Tester', $3, $4, NOW(), 'active', $5, $5)
        "#,
    )
    .bind(principal_id)
    .bind(email)
    .bind(format!("user-{}", principal_id))
    .bind(password_hash)
    .bind(now)
    .execute(pool)
    .await
    .expect("user insert should succeed");

    let cached_session = cached_session_from_login(
        session_id,
        principal_id,
        Some(tenant_id),
        None,
        None,
        None,
        None,
        token_hash("session-token"),
        Some("aal1".to_string()),
        vec!["pwd".to_string()],
        now,
        now,
        None,
        None,
        now + chrono::Duration::hours(2),
    );
    nvbes_redis::session::set_session(redis, &cached_session, current_session_ttl(&cached_session))
        .await
        .expect("redis session insert should succeed");

    let refresh_jti = format!("jti-{}", Uuid::new_v4());
    refresh_token::store_refresh_token(
        redis,
        &CachedRefreshToken {
            jti: refresh_jti,
            session_id,
            principal_id,
            tenant_id: Some(tenant_id),
            organization_id: None,
            workspace_id: None,
            client_id: None,
            scope: "openid profile email offline_access".to_string(),
            authorization_details: vec![],
            expires_at: now + chrono::Duration::days(30),
            rotated_from_jti: None,
            replaced_by_jti: None,
            reuse_detected_at: None,
            last_used_at: None,
            revoked_at: None,
        },
    )
    .await
    .expect("refresh token insert should succeed");

    password_reset::store_password_reset_token(
        redis,
        &CachedPasswordResetToken {
            principal_id,
            token_hash: token_hash(&reset_token),
            created_at: now,
            expires_at: now + chrono::Duration::hours(1),
            consumed_at: None,
        },
    )
    .await
    .expect("reset token insert should succeed");

    (tenant_id, principal_id, session_id, reset_token)
}

async fn cleanup(
    pool: &PgPool,
    redis: &nvbes_redis::RedisPool,
    tenant_id: Uuid,
    principal_id: Uuid,
) {
    let _ = nvbes_redis::session::clear_user_sessions(redis, &principal_id.to_string()).await;
    let _ = nvbes_redis::refresh_token::revoke_all_user_refresh_tokens(redis, principal_id).await;
    sqlx::query("DELETE FROM users WHERE principal_id = $1")
        .bind(principal_id)
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM principals WHERE id = $1")
        .bind(principal_id)
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM tenants WHERE id = $1")
        .bind(tenant_id)
        .execute(pool)
        .await
        .ok();
}

#[tokio::test]
async fn reset_password_updates_credentials_and_revokes_existing_sessions() {
    let pool = test_pool();
    let redis = test_redis_pool().await;
    let (tenant_id, principal_id, session_id, reset_token) = seed_subject(&pool, &redis).await;

    reset(
        &pool,
        &redis,
        &AppConfig::default(),
        ResetPasswordInput {
            token: reset_token.clone(),
            new_password: "N3w$trongPassw0rd!2026".to_string(),
        },
    )
    .await
    .expect("password reset should succeed");

    let user = sqlx::query("SELECT password_hash FROM users WHERE principal_id = $1")
        .bind(principal_id)
        .fetch_one(&pool)
        .await
        .expect("user should exist");
    let password_hash: String = user.get("password_hash");
    verify_password(&password_hash, "N3w$trongPassw0rd!2026")
        .expect("password should have been updated");

    assert!(
        nvbes_redis::session::get_session(&redis, &session_id.to_string())
            .await
            .expect("redis session lookup should succeed")
            .is_none()
    );

    let refresh_token =
        refresh_token::latest_refresh_token_for_session(&redis, principal_id, session_id)
            .await
            .expect("refresh token lookup should succeed")
            .expect("refresh token should exist");
    assert!(refresh_token.revoked_at.is_some());

    cleanup(&pool, &redis, tenant_id, principal_id).await;
}

#[tokio::test]
async fn reset_password_rejects_reused_token() {
    let pool = test_pool();
    let redis = test_redis_pool().await;
    let (tenant_id, principal_id, _, reset_token) = seed_subject(&pool, &redis).await;

    password_reset::mark_password_reset_token_consumed(&redis, &token_hash(&reset_token))
        .await
        .expect("reset token should be markable as used");

    let error = reset(
        &pool,
        &redis,
        &AppConfig::default(),
        ResetPasswordInput {
            token: reset_token,
            new_password: "N3w$trongPassw0rd!2026".to_string(),
        },
    )
    .await
    .expect_err("reused token should be rejected");

    assert_eq!(error.code, "reset_token_expired");

    cleanup(&pool, &redis, tenant_id, principal_id).await;
}
