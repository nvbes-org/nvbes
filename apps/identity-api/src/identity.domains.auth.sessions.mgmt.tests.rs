use super::logout;
use crate::domains::auth::sessions::cache::{cached_session_from_login, current_session_ttl};
use chrono::Utc;
use nvbes_redis::refresh_token::{self, CachedRefreshToken};
use sqlx::PgPool;
use uuid::Uuid;

fn test_pool() -> PgPool {
    crate::test_support::shared_test_pool()
}

async fn test_redis_pool() -> nvbes_redis::RedisPool {
    crate::test_support::test_redis_pool().await
}

async fn seed_subject(pool: &PgPool, redis: &nvbes_redis::RedisPool) -> (Uuid, Uuid, Uuid, Uuid) {
    crate::test_support::ensure_test_database(pool).await;

    let tenant_id = Uuid::new_v4();
    let principal_id = Uuid::new_v4();
    let session_id = Uuid::new_v4();
    let other_session_id = Uuid::new_v4();
    let now = Utc::now();

    sqlx::query(
        r#"
        INSERT INTO tenants (id, kind, name, slug, status, security_tier, created_at, updated_at)
        VALUES ($1, 'personal', 'Logout Tenant', $2, 'active', 'standard', $3, $3)
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
        VALUES ($1, $2, 'human', 'active', 'Logout Tester', $3, $3)
        "#,
    )
    .bind(principal_id)
    .bind(tenant_id)
    .bind(now)
    .execute(pool)
    .await
    .expect("principal insert should succeed");

    for current_session_id in [session_id, other_session_id] {
        let cached_session = cached_session_from_login(
            current_session_id,
            principal_id,
            Some(tenant_id),
            None,
            None,
            None,
            None,
            format!("session-token-{current_session_id}"),
            Some("aal1".to_string()),
            vec!["pwd".to_string()],
            now,
            now,
            None,
            None,
            now + chrono::Duration::hours(2),
        );
        nvbes_redis::session::set_session(
            redis,
            &cached_session,
            current_session_ttl(&cached_session),
        )
        .await
        .expect("redis session insert should succeed");

        refresh_token::store_refresh_token(
            redis,
            &CachedRefreshToken {
                jti: format!("jti-{current_session_id}"),
                session_id: current_session_id,
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
    }

    (tenant_id, principal_id, session_id, other_session_id)
}

async fn cleanup(
    pool: &PgPool,
    redis: &nvbes_redis::RedisPool,
    tenant_id: Uuid,
    principal_id: Uuid,
) {
    let _ = nvbes_redis::session::clear_user_sessions(redis, &principal_id.to_string()).await;
    let _ = nvbes_redis::refresh_token::revoke_all_user_refresh_tokens(redis, principal_id).await;
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
async fn logout_revokes_current_session_and_refresh_token_only() {
    let pool = test_pool();
    let redis = test_redis_pool().await;
    let (tenant_id, principal_id, session_id, other_session_id) = seed_subject(&pool, &redis).await;

    logout(&pool, &redis, session_id, principal_id)
        .await
        .expect("logout should succeed");

    assert!(
        nvbes_redis::session::get_session(&redis, &session_id.to_string())
            .await
            .expect("redis current session lookup should succeed")
            .is_none()
    );

    assert!(
        nvbes_redis::session::get_session(&redis, &other_session_id.to_string())
            .await
            .expect("redis other session lookup should succeed")
            .is_some()
    );

    let current_refresh = refresh_token::get_refresh_token(&redis, &format!("jti-{session_id}"))
        .await
        .expect("current refresh token lookup should succeed")
        .expect("current refresh token should exist");
    assert!(current_refresh.revoked_at.is_some());

    let other_refresh =
        refresh_token::get_refresh_token(&redis, &format!("jti-{other_session_id}"))
            .await
            .expect("other refresh token lookup should succeed")
            .expect("other refresh token should exist");
    assert!(other_refresh.revoked_at.is_none());

    cleanup(&pool, &redis, tenant_id, principal_id).await;
}
